export { ChainIdMismatchError, hasChainIdMismatch } from './chain-id.js';
import { RpcSubscription } from './subscription.js';
export {
  RpcSubscription,
  RpcSubscriptionTimeoutError,
} from './subscription.js';
import {
  parseJson,
  stringifyJson,
  smallInteger,
  type RpcMethod,
  type WireValue,
} from '@open-graphene/codec';

export class RpcRemoteError extends Error {
  constructor(readonly detail: WireValue) {
    super(`RPC rejected request: ${stringifyJson(detail)}`);
    this.name = 'RpcRemoteError';
  }
}
export class RpcTransportError extends Error {
  constructor(
    message: string,
    readonly sent: boolean,
  ) {
    super(message);
    this.name = 'RpcTransportError';
  }
}
interface Pending {
  resolve(value: WireValue): void;
  reject(error: unknown): void;
  timer: ReturnType<typeof setTimeout>;
}
export interface ConnectionOptions {
  /** Cancels connection setup and the entire lifetime of the connection. */
  readonly signal?: AbortSignal;
  readonly timeoutMs?: number;
  readonly createSocket?: (endpoint: string) => WebSocket;
}

/** One Graphene connection. No automatic retry, including broadcasts. */
export class RpcClient {
  #socket: WebSocket;
  #pending = new Map<number, Pending>();
  #apis = new Map<string, Promise<number>>();
  #nextId = 0;
  #timeout: number;
  #callbacks = new Map<number, Set<RpcSubscription>>();
  #databaseCallback: Promise<number> | undefined;
  #closed = false;
  #signal: AbortSignal | undefined;

  private constructor(
    socket: WebSocket,
    timeout: number,
    signal: AbortSignal | undefined,
  ) {
    this.#socket = socket;
    this.#timeout = timeout;
    this.#signal = signal;

    socket.addEventListener('message', this.#onMessage);
    socket.addEventListener('close', this.#onClose);
    socket.addEventListener('error', this.#onError);
    signal?.addEventListener('abort', this.#onAbort, {
      once: true,
    });

    // A custom socket factory can cancel the signal before returning its socket.
    if (signal?.aborted) {
      this.#onAbort();
    }
  }

  #onMessage = (event: MessageEvent): void => {
    try {
      if (typeof event.data !== 'string') {
        throw new Error('Expected text RPC frame');
      }

      const envelope = parseJson(event.data);
      if (
        !envelope ||
        typeof envelope !== 'object' ||
        Array.isArray(envelope)
      ) {
        throw new Error('Invalid RPC envelope');
      }

      const message = envelope as Record<string, WireValue>;
      if (message.method === 'notice') {
        if (!Array.isArray(message.params) || message.params.length !== 2) {
          throw new Error('Invalid callback notice');
        }

        const callback = smallInteger(0, Number.MAX_SAFE_INTEGER).decode(
          message.params[0],
        );
        for (const stream of this.#callbacks.get(callback) ?? []) {
          stream.push(message.params[1]!);
        }

        return;
      }

      if (message.id === undefined) {
        return;
      }

      const id = smallInteger(0, Number.MAX_SAFE_INTEGER).decode(message.id);
      const pending = this.#pending.get(id);
      if (!pending) {
        return;
      }

      this.#pending.delete(id);
      clearTimeout(pending.timer);

      if (message.error !== undefined) {
        pending.reject(new RpcRemoteError(message.error));
      } else if (Object.hasOwn(message, 'result')) {
        pending.resolve(message.result!);
      } else {
        pending.reject(
          new RpcTransportError('RPC response has no result', true),
        );
      }
    } catch {
      this.#shutdown(new RpcTransportError('Malformed RPC response', true));
    }
  };

  #onClose = (): void => {
    this.#shutdown(new RpcTransportError('Connection closed', true));
  };

  #onError = (): void => {
    this.#shutdown(new RpcTransportError('Connection failed', true));
  };

  #onAbort = (): void => {
    this.#shutdown(this.#signal?.reason);
  };

  #shutdown(reason: unknown): void {
    if (this.#closed) {
      return;
    }

    this.#closed = true;
    this.#signal?.removeEventListener('abort', this.#onAbort);
    this.#socket.removeEventListener('message', this.#onMessage);
    this.#socket.removeEventListener('close', this.#onClose);
    this.#socket.removeEventListener('error', this.#onError);

    this.#rejectAll(reason);

    if (this.#socket.readyState < 2) {
      this.#socket.close();
    }
  }

  #rejectAll(error: unknown): void {
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.#pending.clear();

    for (const streams of [...this.#callbacks.values()]) {
      for (const stream of [...streams]) {
        stream.fail(error);
      }
    }
    this.#callbacks.clear();
  }

  static async connect(
    endpoint: string,
    options: ConnectionOptions = {},
  ): Promise<RpcClient> {
    const signal = options.signal;
    signal?.throwIfAborted();

    if (!/^wss?:\/\//.test(endpoint)) {
      throw new Error('Expected WebSocket endpoint');
    }

    const timeout = options.timeoutMs ?? 12000;
    if (!Number.isSafeInteger(timeout) || timeout < 1 || timeout > 2147483647) {
      throw new Error('Invalid RPC timeout');
    }

    const createSocket = options.createSocket ?? ((url) => new WebSocket(url));
    const socket = createSocket(endpoint);
    const client = new RpcClient(socket, timeout, signal);

    try {
      signal?.throwIfAborted();
      await new Promise<void>((resolve, reject) => {
        const cleanup = () => {
          clearTimeout(timer);
          socket.removeEventListener('open', open);
          socket.removeEventListener('error', failed);
          socket.removeEventListener('close', failed);
          signal?.removeEventListener('abort', aborted);
        };
        const open = () => {
          cleanup();
          resolve();
        };
        const fail = (error: unknown) => {
          cleanup();
          reject(error);
        };
        const failed = () => {
          fail(new RpcTransportError('WebSocket connection failed', false));
        };
        const aborted = () => {
          // Do not wait for the peer to acknowledge closing the socket.
          fail(signal?.reason);
        };
        const timer = setTimeout(
          () => fail(new RpcTransportError('Connection timeout', false)),
          timeout,
        );
        socket.addEventListener('open', open);
        socket.addEventListener('error', failed);
        socket.addEventListener('close', failed);
        signal?.addEventListener('abort', aborted, {
          once: true,
        });
      });

      const loggedIn = await client.#call(1, 'login', ['', '']);
      signal?.throwIfAborted();

      if (loggedIn !== true) {
        throw new Error('Anonymous RPC login refused');
      }

      return client;
    } catch (error) {
      client.close();
      signal?.throwIfAborted();
      throw error;
    }
  }

  #call(
    api: number,
    method: string,
    args: readonly WireValue[],
  ): Promise<WireValue> {
    if (this.#signal?.aborted) {
      return Promise.reject(this.#signal.reason);
    }

    if (this.#closed || this.#socket.readyState !== 1) {
      return Promise.reject(new RpcTransportError('Socket is not open', false));
    }

    const id = ++this.#nextId;
    const payload = stringifyJson({
      id,
      method: 'call',
      params: [api, method, args],
    });
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new RpcTransportError(`RPC timeout: ${method}`, true));
      }, this.#timeout);
      this.#pending.set(id, {
        resolve,
        reject,
        timer,
      });

      try {
        this.#socket.send(payload);
      } catch {
        clearTimeout(timer);
        this.#pending.delete(id);
        reject(new RpcTransportError('WebSocket send failed', false));
      }
    });
  }

  async request(
    api: string,
    method: string,
    args: readonly WireValue[],
  ): Promise<WireValue> {
    let discovery = this.#apis.get(api);
    if (!discovery) {
      discovery = this.#call(1, api, []).then((value) =>
        smallInteger(0, 0xffffffff).decode(value),
      );
      this.#apis.set(api, discovery);
    }
    const apiId = await discovery;
    const result = await this.#call(apiId, method, args);
    this.#signal?.throwIfAborted();

    return result;
  }

  async invoke<P, R>(descriptor: RpcMethod<P, R>, params: P): Promise<R> {
    const args = descriptor.encodeParams(params);
    const response = await this.request(
      descriptor.api,
      descriptor.method,
      args,
    );

    return descriptor.parseReturns(response);
  }

  callback(id = ++this.#nextId): RpcSubscription {
    this.#signal?.throwIfAborted();

    if (this.#closed || this.#socket.readyState !== 1) {
      throw new RpcTransportError('Connection closed', false);
    }

    let streams = this.#callbacks.get(id);
    if (!streams) {
      streams = new Set();
      this.#callbacks.set(id, streams);
    }
    const target = streams;
    const stream = new RpcSubscription(id, () => {
      target.delete(stream);
      if (!target.size) {
        this.#callbacks.delete(id);
      }
    });
    target.add(stream);
    return stream;
  }

  async subscribe(
    api: string,
    method: string,
    args: (callbackId: number) => readonly WireValue[],
  ): Promise<RpcSubscription> {
    const stream = this.callback();

    try {
      const params = args(stream.callbackId);
      await this.request(api, method, params);
      this.#signal?.throwIfAborted();
      return stream;
    } catch (error) {
      stream.close();
      throw error;
    }
  }

  async databaseNotices(): Promise<RpcSubscription> {
    if (!this.#databaseCallback) {
      const id = ++this.#nextId;
      this.#databaseCallback = this.request(
        'database',
        'set_subscribe_callback',
        [id, false],
      ).then(() => id);
      this.#databaseCallback.catch(() => {
        this.#databaseCallback = undefined;
      });
    }
    const callbackId = await this.#databaseCallback;

    return this.callback(callbackId);
  }

  close(): void {
    this.#shutdown(new RpcTransportError('Client closed', true));
  }
}

export {
  GrapheneSession,
  type SessionOptions,
  type ReconnectPolicy,
  type ServerLatency,
} from './session.js';
export { ChainStore } from './chain-store.js';

export {
  createSharedConnection,
  type SharedConnection,
  type ConnectionLease,
  type SharedSubscription,
} from './shared-connection.js';
