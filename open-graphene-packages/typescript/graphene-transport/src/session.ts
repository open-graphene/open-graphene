import { ChainIdMismatchError } from './chain-id.js';
import {
  RpcClient,
  RpcTransportError,
  RpcRemoteError,
  type ConnectionOptions,
} from './index.js';
import type { RpcMethod, WireValue } from '@open-graphene/codec';
import type {
  ConnectionLease,
  SharedConnection,
  SharedSubscription,
} from './shared-connection.js';
// Shared leases may come from a separately bundled SDK. Recreate transport
// errors locally so existing retry and broadcast-delivery checks keep their meaning.
function localTransportError(error: unknown): unknown {
  if (!(error instanceof Error)) {
    return error;
  }
  if (
    error.name === 'RpcTransportError' &&
    'sent' in error &&
    typeof error.sent === 'boolean'
  ) {
    return new RpcTransportError(error.message, error.sent);
  }
  if (error.name === 'RpcRemoteError' && 'detail' in error) {
    return new RpcRemoteError(error.detail as WireValue);
  }
  if (
    error.name === 'ChainIdMismatchError' &&
    'expectedChainId' in error &&
    'actualChainId' in error &&
    typeof error.expectedChainId === 'string' &&
    typeof error.actualChainId === 'string'
  ) {
    return new ChainIdMismatchError({
      expectedChainId: error.expectedChainId,
      actualChainId: error.actualChainId,
    });
  }
  return error;
}

export interface ReconnectPolicy {
  readonly maxRetries: number;
  readonly delayMs: number;
}
export interface SessionOptions extends ConnectionOptions {
  readonly connection?: SharedConnection;
  readonly expectedChainId?: string;
  readonly strategy?: 'first-available' | 'lowest-latency';
  readonly reconnect?: ReconnectPolicy;
}
export interface ServerLatency {
  readonly endpoint: string;
  readonly milliseconds: number;
  readonly chainId: string;
}
export class GrapheneSession {
  #client: RpcClient | ConnectionLease;
  #reconnecting: Promise<void> | undefined;
  #policy: ReconnectPolicy;
  #lifetime = new AbortController();
  #signal: AbortSignal;

  private constructor(
    client: RpcClient | ConnectionLease,
    readonly endpoint: string,
    readonly chainId: string,
    private readonly options: SessionOptions,
  ) {
    this.#client = client;
    this.#signal = options.signal
      ? AbortSignal.any([options.signal, this.#lifetime.signal])
      : this.#lifetime.signal;
    this.#policy = options.reconnect ?? {
      maxRetries: 1,
      delayMs: 100,
    };
    this.setReconnectPolicy(this.#policy);
  }

  static async #open(
    endpoint: string,
    options: SessionOptions,
  ): Promise<GrapheneSession> {
    const client = await RpcClient.connect(endpoint, options);

    try {
      const id = await client.request('database', 'get_chain_id', []);
      options.signal?.throwIfAborted();

      if (typeof id !== 'string' || !/^[0-9a-f]{64}$/.test(id)) {
        throw new Error('Invalid chain ID');
      }

      if (options.expectedChainId && id !== options.expectedChainId) {
        throw new ChainIdMismatchError({
          expectedChainId: options.expectedChainId,
          actualChainId: id,
        });
      }

      return new GrapheneSession(client, endpoint, id, options);
    } catch (e) {
      client.close();
      throw e;
    }
  }

  static async connect(
    servers: string | readonly string[],
    options: SessionOptions = {},
  ): Promise<GrapheneSession> {
    options.signal?.throwIfAborted();

    const endpoints = typeof servers === 'string' ? [servers] : [...servers];
    if (!endpoints.length) {
      throw new Error('No RPC endpoints');
    }

    if (options.connection) {
      const lease = await options.connection
        .acquire(options)
        .catch((error: unknown) => {
          throw localTransportError(error);
        });
      return new GrapheneSession(lease, lease.endpoint, lease.chainId, options);
    }

    if (options.strategy === 'lowest-latency') {
      const latencies = await this.probeLatencies(endpoints, options);
      endpoints.splice(
        0,
        endpoints.length,
        ...latencies.map((l) => l.endpoint),
      );
    }

    const errors: unknown[] = [];
    for (const endpoint of endpoints) {
      try {
        const session = await this.#open(endpoint, options);
        options.signal?.throwIfAborted();

        return session;
      } catch (e) {
        options.signal?.throwIfAborted();
        errors.push(e);
      }
    }

    if (errors.length === 1) {
      throw errors[0];
    }

    throw new AggregateError(errors, 'No compatible RPC endpoint available');
  }

  static async probeLatencies(
    servers: readonly string[],
    options: SessionOptions = {},
  ): Promise<ServerLatency[]> {
    options.signal?.throwIfAborted();

    const results = await Promise.allSettled(
      servers.map(async (endpoint) => {
        const start = performance.now();
        const session = await this.#open(endpoint, options);

        try {
          return {
            endpoint,
            milliseconds: performance.now() - start,
            chainId: session.chainId,
          };
        } finally {
          session.close();
        }
      }),
    );
    options.signal?.throwIfAborted();

    const ok = results.flatMap((r) =>
      r.status === 'fulfilled' ? [r.value] : [],
    );
    if (!ok.length) {
      throw new AggregateError(
        results.filter((r) => r.status === 'rejected').map((r) => r.reason),
        'No compatible RPC endpoint available',
      );
    }

    return ok.sort((a, b) => a.milliseconds - b.milliseconds);
  }

  setReconnectPolicy(policy: ReconnectPolicy): void {
    if (
      !Number.isSafeInteger(policy.maxRetries) ||
      policy.maxRetries < 0 ||
      policy.maxRetries > 10 ||
      !Number.isSafeInteger(policy.delayMs) ||
      policy.delayMs < 0 ||
      policy.delayMs > 60000
    ) {
      throw new Error('Invalid reconnect policy');
    }

    this.#policy = {
      ...policy,
    };
  }

  async reconnect(): Promise<void> {
    this.#signal.throwIfAborted();
    if (this.#reconnecting) {
      return this.#reconnecting;
    }

    if ('reconnect' in this.#client) {
      try {
        await this.#client.reconnect();
      } catch (error) {
        throw localTransportError(error);
      }
      return;
    }

    this.#reconnecting = (async () => {
      // Retire the previous socket before opening its replacement.
      this.#client.close();
      const next = await GrapheneSession.#open(this.endpoint, {
        ...this.options,
        signal: this.#signal,
        expectedChainId: this.chainId,
      });
      this.#signal.throwIfAborted();
      this.#client.close();
      this.#client = next.#client;
    })();

    try {
      await this.#reconnecting;
    } finally {
      this.#reconnecting = undefined;
    }
  }

  async request(
    api: string,
    method: string,
    args: readonly WireValue[],
  ): Promise<WireValue> {
    this.#signal.throwIfAborted();
    // Raw requests, broadcasts and callback registration are never retried.
    try {
      return await this.#client.request(api, method, args);
    } catch (error) {
      throw localTransportError(error);
    }
  }

  async invoke<P, R>(descriptor: RpcMethod<P, R>, params: P): Promise<R> {
    const read = /^(get_|lookup_|list_)/.test(descriptor.method);
    for (let attempt = 0; ; attempt++) {
      try {
        const paramsOnWire = descriptor.encodeParams(params);
        const response = await this.request(
          descriptor.api,
          descriptor.method,
          paramsOnWire,
        );
        this.#signal.throwIfAborted();

        return descriptor.parseReturns(response);
      } catch (e) {
        this.#signal.throwIfAborted();

        if (
          !read ||
          !(e instanceof RpcTransportError) ||
          attempt >= this.#policy.maxRetries
        ) {
          throw e;
        }

        if (this.#policy.delayMs) {
          await this.#waitToReconnect();
        }

        await this.reconnect();
      }
    }
  }

  #waitToReconnect(): Promise<void> {
    this.#signal.throwIfAborted();

    return new Promise((resolve, reject) => {
      const cleanup = () => {
        clearTimeout(timer);
        this.#signal.removeEventListener('abort', aborted);
      };
      const aborted = () => {
        cleanup();
        reject(this.#signal.reason);
      };
      const timer = setTimeout(() => {
        cleanup();
        resolve();
      }, this.#policy.delayMs);

      this.#signal.addEventListener('abort', aborted, {
        once: true,
      });
    });
  }

  async subscribe(
    api: string,
    method: string,
    args: (id: number) => readonly WireValue[],
  ): Promise<SharedSubscription> {
    this.#signal.throwIfAborted();
    try {
      return await this.#client.subscribe(api, method, args);
    } catch (error) {
      throw localTransportError(error);
    }
  }

  async databaseNotices(): Promise<SharedSubscription> {
    this.#signal.throwIfAborted();
    return this.#client.databaseNotices();
  }

  close(): void {
    this.#lifetime.abort(new Error('Session closed'));
    this.#client.close();
  }
}
