import { parseJson, stringifyJson, smallInteger, type RpcMethod, type WireValue } from '@open-graphene/codec';

export class RpcRemoteError extends Error {
  constructor(readonly detail: WireValue) { super(`RPC rejected request: ${stringifyJson(detail)}`); this.name = 'RpcRemoteError'; }
}
export class RpcTransportError extends Error {
  constructor(message: string, readonly sent: boolean) { super(message); this.name = 'RpcTransportError'; }
}
interface Pending { resolve(value: WireValue): void; reject(error: Error): void; timer: ReturnType<typeof setTimeout> }
export interface ConnectionOptions {
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
  private constructor(socket: WebSocket, timeout: number) {
    this.#socket = socket; this.#timeout = timeout;
    socket.addEventListener('message', event => {
      try {
        if (typeof event.data !== 'string') throw new Error('Expected text RPC frame');
        const envelope = parseJson(event.data);
        if (!envelope || typeof envelope !== 'object' || Array.isArray(envelope)) throw new Error('Invalid RPC envelope');
        const message = envelope as Record<string, WireValue>;
        if (message.id === undefined) return; // Notice support is a separate stage.
        const id = smallInteger(0, Number.MAX_SAFE_INTEGER).decode(message.id);
        const pending = this.#pending.get(id);
        if (!pending) return;
        this.#pending.delete(id); clearTimeout(pending.timer);
        if (message.error !== undefined) pending.reject(new RpcRemoteError(message.error));
        else if (Object.hasOwn(message, 'result')) pending.resolve(message.result!);
        else pending.reject(new RpcTransportError('RPC response has no result', true));
      } catch { this.#rejectAll(new RpcTransportError('Malformed RPC response', true)); this.#socket.close(); }
    });
    socket.addEventListener('close', () => this.#rejectAll(new RpcTransportError('Connection closed', true)));
    socket.addEventListener('error', () => this.#rejectAll(new RpcTransportError('Connection failed', true)));
  }
  #rejectAll(error: Error): void {
    for (const p of this.#pending.values()) { clearTimeout(p.timer); p.reject(error); }
    this.#pending.clear();
  }
  static async connect(endpoint: string, options: ConnectionOptions = {}): Promise<RpcClient> {
    if (!/^wss?:\/\//.test(endpoint)) throw new Error('Expected WebSocket endpoint');
    const timeout = options.timeoutMs ?? 12000;
    if (!Number.isSafeInteger(timeout) || timeout < 1 || timeout > 2147483647) throw new Error('Invalid RPC timeout');
    const socket = (options.createSocket ?? (url => new WebSocket(url)))(endpoint);
    const client = new RpcClient(socket, timeout);
    try {
      await new Promise<void>((resolve, reject) => {
        const finish = (error?: Error) => {
          clearTimeout(timer); socket.removeEventListener('open', open); socket.removeEventListener('error', failed); socket.removeEventListener('close', failed);
          error ? reject(error) : resolve();
        };
        const open = () => finish();
        const failed = () => finish(new RpcTransportError('WebSocket connection failed', false));
        const timer = setTimeout(() => finish(new RpcTransportError('Connection timeout', false)), timeout);
        socket.addEventListener('open', open); socket.addEventListener('error', failed); socket.addEventListener('close', failed);
      });
      if (await client.#call(1, 'login', ['', '']) !== true) throw new Error('Anonymous RPC login refused');
      return client;
    } catch (error) { client.close(); throw error; }
  }
  #call(api: number, method: string, args: readonly WireValue[]): Promise<WireValue> {
    if (this.#socket.readyState !== 1) return Promise.reject(new RpcTransportError('Socket is not open', false));
    const id = ++this.#nextId;
    const payload = stringifyJson({ id, method: 'call', params: [api, method, args] });
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.#pending.delete(id); reject(new RpcTransportError(`RPC timeout: ${method}`, true)); }, this.#timeout);
      this.#pending.set(id, { resolve, reject, timer });
      try { this.#socket.send(payload); }
      catch { clearTimeout(timer); this.#pending.delete(id); reject(new RpcTransportError('WebSocket send failed', false)); }
    });
  }
  async request(api: string, method: string, args: readonly WireValue[]): Promise<WireValue> {
    let discovery = this.#apis.get(api);
    if (!discovery) {
      discovery = this.#call(1, api, []).then(value => smallInteger(0, 0xffffffff).decode(value));
      this.#apis.set(api, discovery);
    }
    return this.#call(await discovery, method, args);
  }
  async invoke<P, R>(descriptor: RpcMethod<P, R>, params: P): Promise<R> {
    const args = descriptor.encodeParams(params);
    return descriptor.parseReturns(await this.request(descriptor.api, descriptor.method, args));
  }
  close(): void { this.#rejectAll(new RpcTransportError('Client closed', true)); this.#socket.close(); }
}
