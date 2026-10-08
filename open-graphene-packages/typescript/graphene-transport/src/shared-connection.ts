import { GrapheneSession, type SessionOptions } from './session.js';
import { ChainIdMismatchError } from './chain-id.js';
import type { WireValue } from '@open-graphene/codec';
import type { RpcSubscription } from './subscription.js';

/** Public operations only, so independently bundled SDKs can borrow the connection. */
export type SharedSubscription = Pick<RpcSubscription, keyof RpcSubscription>;

export interface ConnectionLease {
  readonly endpoint: string;
  readonly chainId: string;
  request(
    api: string,
    method: string,
    args: readonly WireValue[],
  ): Promise<WireValue>;
  subscribe(
    api: string,
    method: string,
    args: (id: number) => readonly WireValue[],
  ): Promise<SharedSubscription>;
  databaseNotices(): Promise<SharedSubscription>;
  reconnect(): Promise<void>;
  close(): void;
}

export interface SharedConnection {
  acquire(options: {
    expectedChainId?: string;
    signal?: AbortSignal;
    timeoutMs?: number;
  }): Promise<ConnectionLease>;
  close(): void;
}

function waitFor<T>(work: Promise<T>, signal: AbortSignal): Promise<T> {
  return new Promise((resolve, reject) => {
    const aborted = () => reject(signal.reason);
    signal.addEventListener('abort', aborted, {
      once: true,
    });
    if (signal.aborted) {
      aborted();
    }

    work.then(resolve, reject).finally(() => {
      signal.removeEventListener('abort', aborted);
    });
  });
}

/** One lazy connection, owned by the host. Leases only own their cancellation and subscriptions. */
export function createSharedConnection(options: {
  endpoints: readonly string[];
  expectedChainId: string;
  timeoutMs?: number;
  createSocket?: SessionOptions['createSocket'];
}): SharedConnection {
  const endpoints = [...options.endpoints];
  const chainId = options.expectedChainId;
  const timeoutMs = options.timeoutMs ?? 10000;
  const createSocket = options.createSocket;
  const lifetime = new AbortController();
  let session: GrapheneSession | undefined;
  let opening: Promise<GrapheneSession> | undefined;
  let reconnecting: Promise<void> | undefined;

  async function open(): Promise<GrapheneSession> {
    lifetime.signal.throwIfAborted();

    if (session) {
      return session;
    }

    if (!opening) {
      opening = GrapheneSession.connect(endpoints, {
        expectedChainId: chainId,
        timeoutMs,
        ...(createSocket
          ? {
              createSocket,
            }
          : {}),
        signal: lifetime.signal,
        reconnect: {
          maxRetries: 0,
          delayMs: 0,
        },
      })
        .then((connected) => {
          if (lifetime.signal.aborted) {
            connected.close();
            lifetime.signal.throwIfAborted();
          }

          session = connected;
          return connected;
        })
        .finally(() => {
          opening = undefined;
        });
    }

    return opening;
  }

  async function reconnect(): Promise<void> {
    lifetime.signal.throwIfAborted();

    if (!reconnecting) {
      reconnecting = (async () => {
        const connected = await open();
        await connected.reconnect();
      })().finally(() => {
        reconnecting = undefined;
      });
    }

    await reconnecting;
  }

  return {
    async acquire(input) {
      if (input.expectedChainId && input.expectedChainId !== chainId) {
        throw new ChainIdMismatchError({
          expectedChainId: input.expectedChainId,
          actualChainId: chainId,
        });
      }

      const local = new AbortController();
      const signals = [lifetime.signal, local.signal];
      if (input.signal) {
        signals.push(input.signal);
      }

      const signal = AbortSignal.any(signals);
      signal.throwIfAborted();
      const connected = await waitFor(open(), signal);
      signal.throwIfAborted();

      const streams = new Set<SharedSubscription>();
      const release = () => {
        for (const stream of streams) {
          stream.close();
        }
        streams.clear();
      };
      signal.addEventListener('abort', release, { once: true });

      function bounded<T>(work: Promise<T>): Promise<T> {
        const deadline = AbortSignal.timeout(input.timeoutMs ?? timeoutMs);
        return waitFor(work, AbortSignal.any([signal, deadline]));
      }

      async function retain(work: Promise<SharedSubscription>) {
        const deadline = AbortSignal.timeout(input.timeoutMs ?? timeoutMs);
        const subscriptionSignal = AbortSignal.any([signal, deadline]);
        const retained = work.then((stream) => {
          if (subscriptionSignal.aborted) {
            stream.close();
            subscriptionSignal.throwIfAborted();
          }

          // Release completed subscriptions immediately, rather than retaining
          // every broadcast callback until the host shuts down.
          const tracked: SharedSubscription = {
            callbackId: stream.callbackId,
            push: (value) => stream.push(value),
            fail(error) {
              streams.delete(tracked);
              stream.fail(error);
            },
            next: () => stream.next(),
            nextTimeout: (milliseconds) => stream.nextTimeout(milliseconds),
            async return() {
              tracked.close();
              return {
                done: true,
                value: undefined,
              };
            },
            close() {
              streams.delete(tracked);
              stream.close();
            },
            [Symbol.asyncIterator]() {
              return tracked;
            },
          };

          streams.add(tracked);
          return tracked;
        });

        return waitFor(retained, subscriptionSignal);
      }

      return {
        endpoint: connected.endpoint,
        chainId: connected.chainId,
        request(api, method, args) {
          signal.throwIfAborted();
          return bounded(connected.request(api, method, args));
        },
        subscribe(api, method, args) {
          signal.throwIfAborted();
          return retain(connected.subscribe(api, method, args));
        },
        databaseNotices() {
          signal.throwIfAborted();
          return retain(connected.databaseNotices());
        },
        reconnect() {
          signal.throwIfAborted();
          return bounded(reconnect());
        },
        close() {
          local.abort(new Error('Connection lease closed'));
          signal.removeEventListener('abort', release);
        },
      };
    },
    close() {
      lifetime.abort(new Error('Shared connection closed'));
      session?.close();
      session = undefined;
    },
  };
}
