import type { WireValue } from '@open-graphene/codec';
export class RpcSubscriptionTimeoutError extends Error {
  readonly name = 'RpcSubscriptionTimeoutError';

  constructor() {
    super('Subscription timeout');
  }
}

/** Bounded callback stream. Overflow fails explicitly rather than losing updates silently. */
export class RpcSubscription<
  T = WireValue,
> implements AsyncIterableIterator<T> {
  #queue: T[] = [];
  #waiters: {
    resolve: (v: IteratorResult<T>) => void;
    reject: (e: unknown) => void;
  }[] = [];
  #error: unknown;
  #failed = false;
  #closed = false;
  constructor(
    readonly callbackId: number,
    private readonly release: () => void,
    private readonly capacity = 1024,
  ) {}
  push(value: T): void {
    if (this.#closed) {
      return;
    }

    const pending = this.#waiters.shift();
    if (pending) {
      pending.resolve({ done: false, value });
      return;
    }
    if (this.#queue.length >= this.capacity) {
      this.fail(new Error('Subscription buffer overflow; resnapshot required'));
      return;
    }
    this.#queue.push(value);
  }
  fail(error: unknown): void {
    if (this.#closed) {
      return;
    }

    this.#error = error;
    this.#failed = true;
    this.#queue = [];
    this.#closed = true;
    this.release();

    for (const waiter of this.#waiters.splice(0)) {
      waiter.reject(error);
    }
  }
  next(): Promise<IteratorResult<T>> {
    if (this.#queue.length) {
      return Promise.resolve({
        done: false,
        value: this.#queue.shift()!,
      });
    }

    if (this.#failed) {
      return Promise.reject(this.#error);
    }

    if (this.#closed) {
      return Promise.resolve({
        done: true,
        value: undefined,
      });
    }

    return new Promise((resolve, reject) => {
      this.#waiters.push({
        resolve,
        reject,
      });
    });
  }
  async nextTimeout(milliseconds: number): Promise<IteratorResult<T>> {
    if (!Number.isSafeInteger(milliseconds) || milliseconds < 1) {
      throw new Error('Invalid subscription timeout');
    }

    if (this.#queue.length || this.#closed) {
      return this.next();
    }

    return new Promise((resolve, reject) => {
      const waiter = {
        resolve: (v: IteratorResult<T>) => {
          clearTimeout(timer);
          resolve(v);
        },
        reject: (e: unknown) => {
          clearTimeout(timer);
          reject(e);
        },
      };
      const timer = setTimeout(() => {
        const i = this.#waiters.indexOf(waiter);
        if (i >= 0) this.#waiters.splice(i, 1);
        reject(new RpcSubscriptionTimeoutError());
      }, milliseconds);
      this.#waiters.push(waiter);
    });
  }
  async return(): Promise<IteratorResult<T>> {
    this.close();
    return { done: true, value: undefined };
  }
  close(): void {
    if (this.#closed) {
      return;
    }

    this.#closed = true;
    this.#queue = [];
    this.release();

    for (const w of this.#waiters.splice(0))
      w.resolve({ done: true, value: undefined });
  }
  [Symbol.asyncIterator](): AsyncIterableIterator<T> {
    return this;
  }
}
