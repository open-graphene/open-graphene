import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { getEventListeners } from 'node:events';
import {
  SwaplockClient,
  SignedTransfer,
  TransactionBroadcastError,
  BroadcastOutcomeUnknown,
} from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import {
  RpcSubscription,
  RpcSubscriptionTimeoutError,
  RpcRemoteError,
  RpcTransportError,
} from '../graphene-transport/dist/index.js';

const vectors = JSON.parse(
  readFileSync(new URL('fixtures/protocol-vectors.json', import.meta.url)),
);

function transaction(expiration = Date.now() + 60000) {
  const unsigned = b.TransactionCodec.decode({
    ...vectors.transfer.transaction,
    expiration: new Date(expiration).toISOString().slice(0, 19),
  });
  return new SignedTransfer(
    {
      ...unsigned,
      signatures: [new Uint8Array(65)],
    },
    100,
  );
}

class Socket extends EventTarget {
  readyState = 0;
  requests = [];
  broadcasts = [];

  constructor(options = {}) {
    super();
    this.options = options;
    queueMicrotask(() => {
      this.readyState = 1;
      this.dispatchEvent(new Event('open'));
    });
  }

  send(raw) {
    const request = JSON.parse(raw);
    const method = request.params[1];
    this.requests.push(request);
    this.dispatchEvent(new Event('request'));

    if (method === 'broadcast_transaction_with_callback') {
      this.broadcasts.push(request);
      this.options.onBroadcast?.();
      if (this.options.holdRegistration) {
        return;
      }

      queueMicrotask(() => {
        if (this.options.reject) {
          this.message({
            id: request.id,
            error: {
              code: 1,
              message: 'rejected',
            },
          });
          return;
        }

        if (this.options.noticeBeforeAck) {
          this.notice(this.confirmation());
        }
        this.message({
          id: request.id,
          result: null,
        });
        if (!this.options.noNotice && !this.options.noticeBeforeAck) {
          this.notice(this.confirmation());
        }
      });
      return;
    }

    let result = 2;
    if (method === 'login') {
      result = true;
    } else if (method === 'get_chain_id') {
      result = b.CHAIN.chainId;
    }
    queueMicrotask(() =>
      this.message({
        id: request.id,
        result,
      }),
    );
  }

  confirmation() {
    const request = this.broadcasts[0];
    const value = {
      id: this.expectedId,
      block_num: 101,
      trx_num: 0,
      trx: {
        ...request.params[2][1],
        operation_results: [[0, {}]],
      },
    };
    return this.options.transform ? this.options.transform(value) : [value];
  }

  notice(value) {
    this.message({
      method: 'notice',
      params: [this.broadcasts[0].params[2][0], value],
    });
  }

  message(value) {
    this.dispatchEvent(
      new MessageEvent('message', {
        data: JSON.stringify(value),
      }),
    );
  }

  close() {
    this.readyState = 3;
    this.dispatchEvent(new Event('close'));
  }
}

async function connect(t, options = {}) {
  const signed = transaction();
  const socket = new Socket(options);
  socket.expectedId = signed.id;
  const controller = new AbortController();
  let connections = 0;
  const client = await SwaplockClient.connect('ws://fixture', {
    expectedChainId: b.CHAIN.chainId,
    signal: controller.signal,
    timeoutMs: 25,
    reconnect: {
      maxRetries: 2,
      delayMs: 0,
    },
    createSocket() {
      connections += 1;
      return socket;
    },
  });
  t.after(() => client.close());
  let released = 0;
  const subscribe = client.rpc.subscribe.bind(client.rpc);
  client.rpc.subscribe = async (...args) => {
    const stream = await subscribe(...args);
    const close = stream.close.bind(stream);
    stream.close = () => {
      released += 1;
      close();
    };
    return stream;
  };

  return {
    client,
    signed,
    socket,
    controller,
    connections: () => connections,
    released: () => released,
  };
}

function broadcastError(code) {
  return (error) =>
    error instanceof TransactionBroadcastError && error.code === code;
}

for (const noticeBeforeAck of [false, true]) {
  test(`callback validation decodes the exact transaction with noticeBeforeAck=${noticeBeforeAck}`, async (t) => {
    const state = await connect(t, {
      noticeBeforeAck,
    });
    const pending =
      await state.client.networkBroadcast.sendTransactionWithCallback(
        state.signed,
      );
    const result = await pending.wait(50);

    assert.equal(result.id, state.signed.id);
    assert.equal(result.blockNumber, 101);
    assert.equal(result.transactionIndex, 0);
    assert.equal(result.transaction.operations[0][1].amount.amount, 100000n);
    assert.equal(state.released(), 1);
    assert.equal(state.socket.broadcasts.length, 1);
    assert.equal(state.connections(), 1);
  });
}

const malformed = [
  [
    'wrong ID',
    (value) => ({
      ...value,
      id: 'ff'.repeat(20),
    }),
    'transaction-mismatch',
  ],
  [
    'zero block',
    (value) => ({
      ...value,
      block_num: 0,
    }),
    'invalid-confirmation',
  ],
  [
    'negative index',
    (value) => ({
      ...value,
      trx_num: -1,
    }),
    'invalid-confirmation',
  ],
  [
    'fractional block',
    (value) => ({
      ...value,
      block_num: 1.5,
    }),
    'invalid-confirmation',
  ],
  [
    'overflow index',
    (value) => ({
      ...value,
      trx_num: 4294967296,
    }),
    'invalid-confirmation',
  ],
  [
    'missing body',
    (value) => ({
      ...value,
      trx: null,
    }),
    'invalid-confirmation',
  ],
  [
    'missing results',
    (value) => ({
      ...value,
      trx: {
        ...value.trx,
        operation_results: [],
      },
    }),
    'invalid-confirmation',
  ],
  ['empty notice', () => [], 'invalid-confirmation'],
  ['extra notice', (value) => [value, value], 'invalid-confirmation'],
  ['null notice', () => null, 'invalid-confirmation'],
  [
    'different body with matching ID',
    (value) => ({
      ...value,
      trx: {
        ...value.trx,
        ref_block_num: 999,
      },
    }),
    'transaction-mismatch',
  ],
  [
    'different signatures',
    (value) => ({
      ...value,
      trx: {
        ...value.trx,
        signatures: ['11'.repeat(65)],
      },
    }),
    'transaction-mismatch',
  ],
];
for (const [name, transform, code] of malformed) {
  test(`${name} is rejected and releases the callback without another send`, async (t) => {
    const state = await connect(t, {
      transform,
    });
    const pending =
      await state.client.networkBroadcast.sendTransactionWithCallback(
        state.signed,
      );

    await assert.rejects(pending.wait(50), broadcastError(code));
    assert.equal(state.released(), 1);
    assert.equal(state.socket.broadcasts.length, 1);
    assert.equal(state.connections(), 1);
  });
}

test('confirmation timeout and a late notice never trigger another send', async (t) => {
  const state = await connect(t, {
    noNotice: true,
  });
  const pending =
    await state.client.networkBroadcast.sendTransactionWithCallback(
      state.signed,
    );

  await assert.rejects(pending.wait(5), broadcastError('timeout'));
  assert.equal(state.released(), 1);
  state.socket.notice(state.socket.confirmation());
  await assert.rejects(pending.wait(50), broadcastError('closed'));
  assert.equal(state.socket.broadcasts.length, 1);
  assert.equal(state.connections(), 1);
});

test('explicit callback close settles a pending wait without another send', async (t) => {
  const state = await connect(t, {
    noNotice: true,
  });
  const pending =
    await state.client.networkBroadcast.sendTransactionWithCallback(
      state.signed,
    );
  const rejected = assert.rejects(pending.wait(500), broadcastError('closed'));
  pending.close();
  await rejected;
  assert.equal(state.socket.broadcasts.length, 1);
});

for (const action of ['cancel', 'disconnect']) {
  test(`${action} while waiting leaves an unknown outcome and never reconnects`, async (t) => {
    const state = await connect(t, {
      noNotice: true,
    });
    const pending =
      await state.client.networkBroadcast.sendTransactionWithCallback(
        state.signed,
      );
    const rejected = assert.rejects(pending.wait(500), BroadcastOutcomeUnknown);
    if (action === 'cancel') {
      state.controller.abort(new Error('selection ended'));
    } else {
      state.socket.close();
    }
    await rejected;
    assert.equal(state.released(), 1);
    assert.equal(state.socket.broadcasts.length, 1);
    assert.equal(state.connections(), 1);
    assert.equal(getEventListeners(state.socket, 'message').length, 0);
  });
}

test('lost registration acknowledgement leaves an unknown outcome without retry', async (t) => {
  const state = await connect(t, {
    holdRegistration: true,
  });
  await assert.rejects(
    state.client.networkBroadcast.sendTransactionWithCallback(state.signed),
    BroadcastOutcomeUnknown,
  );
  assert.equal(state.socket.broadcasts.length, 1);
  assert.equal(state.connections(), 1);
});

test('remote rejection retains its RPC classification without retry', async (t) => {
  const state = await connect(t, {
    reject: true,
  });
  await assert.rejects(
    state.client.networkBroadcast.sendTransactionWithCallback(state.signed),
    RpcRemoteError,
  );
  assert.equal(state.socket.broadcasts.length, 1);
  assert.equal(state.connections(), 1);
});

test('expired transactions and disconnected sockets do not send', async (t) => {
  const state = await connect(t);
  await assert.rejects(
    state.client.networkBroadcast.sendTransactionWithCallback(
      transaction(Date.now() - 1000),
    ),
    broadcastError('expired'),
  );
  state.socket.close();
  await assert.rejects(
    state.client.networkBroadcast.sendTransactionWithCallback(state.signed),
    (error) => error instanceof RpcTransportError && !error.sent,
  );
  assert.equal(state.socket.broadcasts.length, 0);
});

test('subscription timeouts have a typed error and leave the stream available until its owner closes it', async () => {
  let releases = 0;
  const stream = new RpcSubscription(1, () => {
    releases += 1;
  });
  await assert.rejects(stream.nextTimeout(1), RpcSubscriptionTimeoutError);
  stream.push('later');
  assert.deepEqual(await stream.next(), {
    done: false,
    value: 'later',
  });
  stream.close();
  assert.equal(releases, 1);
});
