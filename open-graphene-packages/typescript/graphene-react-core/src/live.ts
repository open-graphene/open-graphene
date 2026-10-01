import { QueryClient, type QueryKey } from '@tanstack/react-query';
import { RpcTransportError } from '@open-graphene/transport';
export interface LiveStream<T> extends AsyncIterable<T> {
  close(): void;
}
export type LiveStatus =
  'disabled' | 'connecting' | 'live' | 'reconnecting' | 'error';
export interface LiveState {
  readonly status: LiveStatus;
  readonly error: Error | null;
}
export const disabledLive: LiveState = Object.freeze({
  status: 'disabled',
  error: null,
});
export interface LivePolicy {
  readonly maxRetries?: number;
  readonly delayMs?: number;
}

/** One subscription per QueryClient + SDK client + query key, reference-counted across hooks. */
export class LiveEntry<T> {
  #refs = 0;
  #generation = 0;
  #stream: LiveStream<T> | undefined;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #listeners = new Set<() => void>();
  #state: LiveState = disabledLive;
  #retries = 0;
  constructor(
    private readonly cache: QueryClient,
    readonly key: QueryKey,
    private readonly open: () => Promise<LiveStream<T>>,
    private readonly reconnect: () => Promise<void>,
    private readonly policy: LivePolicy = {},
  ) {
    if (
      !Number.isInteger(policy.maxRetries ?? 3) ||
      (policy.maxRetries ?? 3) < 0 ||
      (policy.maxRetries ?? 3) > 10
    )
      throw new Error('Invalid live retry limit');
    if (
      !Number.isInteger(policy.delayMs ?? 250) ||
      (policy.delayMs ?? 250) < 0 ||
      (policy.delayMs ?? 250) > 60000
    )
      throw new Error('Invalid live retry delay');
  }
  getSnapshot = (): LiveState => this.#state;
  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };
  #set(status: LiveStatus, error: Error | null = null) {
    this.#state = { status, error };
    for (const listener of this.#listeners) listener();
  }
  acquire(): () => void {
    if (++this.#refs === 1 && this.#state.status === 'disabled') {
      this.#retries = 0;
      this.#set('connecting');
      void this.#run(++this.#generation);
    }
    let released = false;
    return () => {
      if (released) return;
      released = true;
      if (--this.#refs === 0)
        queueMicrotask(() => {
          // React Strict Mode's immediate re-acquire keeps the same subscription.
          if (this.#refs) return;
          ++this.#generation;
          clearTimeout(this.#timer);
          this.#stream?.close();
          this.#stream = undefined;
          this.#set('disabled');
        });
    };
  }
  restart = (): void => {
    if (!this.#refs) return;
    ++this.#generation;
    clearTimeout(this.#timer);
    this.#stream?.close();
    this.#stream = undefined;
    this.#retries = 0;
    this.#set('connecting');
    void this.#run(this.#generation);
  };
  async #run(generation: number): Promise<void> {
    let stream: LiveStream<T> | undefined;
    let error: Error | null = null;
    try {
      stream = await this.open();
      if (generation !== this.#generation || !this.#refs) {
        stream.close();
        return;
      }
      this.#stream = stream;
      for await (const snapshot of stream) {
        if (generation !== this.#generation || !this.#refs) break;
        // Discard a pending stale read before applying the newer subscription snapshot.
        await this.cache.cancelQueries(
          { queryKey: this.key, exact: true },
          { silent: true },
        );
        if (generation !== this.#generation || !this.#refs) break;
        this.cache.setQueryData(this.key, snapshot);
        this.#retries = 0;
        this.#set('live');
      }
    } catch (e) {
      error = e instanceof Error ? e : new Error(String(e));
    } finally {
      stream?.close();
      if (this.#stream === stream) this.#stream = undefined;
    }
    this.#recover(generation, error);
  }
  #recover(generation: number, error: Error | null): void {
    if (generation !== this.#generation || !this.#refs) return;
    // RPC permissions/codec failures require intervention; disconnects and overflow need resnapshot.
    const recoverable =
      !error ||
      error instanceof RpcTransportError ||
      error.message.includes('resnapshot required');
    if (!recoverable || this.#retries >= (this.policy.maxRetries ?? 3)) {
      this.#set('error', error ?? new Error('Subscription ended'));
      return;
    }
    this.#set('reconnecting', error);
    const delay = Math.min(
      (this.policy.delayMs ?? 250) * 2 ** this.#retries++,
      60000,
    );
    this.#timer = setTimeout(() => {
      void (async () => {
        if (generation !== this.#generation || !this.#refs) return;
        try {
          if (error instanceof RpcTransportError) await this.reconnect();
          if (generation === this.#generation && this.#refs)
            await this.#run(generation);
        } catch (e) {
          this.#recover(
            generation,
            e instanceof Error ? e : new Error(String(e)),
          );
        }
      })();
    }, delay);
  }
}
const registries = new WeakMap<
  QueryClient,
  WeakMap<object, Map<string, LiveEntry<unknown>>>
>();
export function sharedLive<T>(
  cache: QueryClient,
  client: object,
  key: QueryKey,
  open: () => Promise<LiveStream<T>>,
  reconnect: () => Promise<void>,
  policy: LivePolicy,
): LiveEntry<T> {
  let clients = registries.get(cache);
  if (!clients) {
    clients = new WeakMap();
    registries.set(cache, clients);
  }
  let entries = clients.get(client);
  if (!entries) {
    entries = new Map();
    clients.set(client, entries);
  }
  const id = JSON.stringify([key, policy]);
  let entry = entries.get(id);
  if (!entry) {
    entry = new LiveEntry(cache, key, open, reconnect, policy);
    entries.set(id, entry);
    // Remove entries once Query itself is collected; no strong references to SDK clients globally.
    const unsubscribe = cache.getQueryCache().subscribe((event) => {
      if (
        event.type === 'removed' &&
        JSON.stringify(event.query.queryKey) === JSON.stringify(key) &&
        entry!.getSnapshot().status === 'disabled'
      ) {
        entries!.delete(id);
        unsubscribe();
      }
    });
  }
  return entry as LiveEntry<T>;
}
