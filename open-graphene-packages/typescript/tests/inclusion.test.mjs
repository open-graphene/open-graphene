import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { getEventListeners } from 'node:events';
import { createHash } from 'node:crypto';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import {
  SwaplockClient,
  transactionId,
  TransactionInspectionError,
} from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';

const vectors = JSON.parse(
  readFileSync(new URL('fixtures/protocol-vectors.json', import.meta.url)),
);
const now = Date.UTC(2026, 9, 8, 12);
const time = (offset = 0) =>
  new Date(now + offset * 1000).toISOString().slice(0, 19);

class Socket extends EventTarget {
  readyState = 0;
  reads = [];
  head = {
    id: '2.1.0',
    head_block_number: 100,
    head_block_id: '00'.repeat(20),
    time: time(),
    current_witness: '1.6.0',
    next_maintenance_time: time(),
    last_vote_tally_time: time(),
    last_budget_time: time(),
    witness_budget: '0',
    total_pob: '0',
    total_inactive: '0',
    accounts_registered_this_interval: 0,
    recently_missed_count: 0,
    current_aslot: '100',
    recent_slots_filled: '0',
    dynamic_flags: 0,
    last_irreversible_block_num: 99,
    maintenance_seed: '0',
  };
  block = {
    previous: '11'.repeat(20),
    timestamp: time(),
    witness: '1.6.0',
    transaction_merkle_root: '22'.repeat(20),
    extensions: [],
    witness_signature: '33'.repeat(65),
    transactions: [
      {
        ...vectors.transfer.transaction,
        signatures: ['44'.repeat(65)],
        operation_results: [[0, {}]],
      },
    ],
  };

  constructor() {
    super();
    queueMicrotask(() => {
      this.readyState = 1;
      this.dispatchEvent(new Event('open'));
    });
  }

  send(raw) {
    const request = JSON.parse(raw);
    const method = request.params[1];
    let result = 2;
    if (method === 'login') {
      result = true;
    } else if (method === 'get_chain_id') {
      result = b.CHAIN.chainId;
    } else if (method === 'get_dynamic_global_properties') {
      this.reads.push(method);
      this.onHead?.();
      result = this.head;
    } else if (method === 'get_block') {
      this.reads.push(method);
      assert.equal(request.params[2][0], 100);
      this.onBlock?.();
      if (this.holdBlock) {
        return;
      }
      result = this.block;
    }

    const data = JSON.stringify({
      id: request.id,
      result,
    });
    queueMicrotask(() =>
      this.dispatchEvent(
        new MessageEvent('message', {
          data,
        }),
      ),
    );
  }

  close() {
    this.readyState = 3;
    this.dispatchEvent(new Event('close'));
  }
}

async function fixture(t) {
  t.mock.method(Date, 'now', () => now);
  const socket = new Socket();
  const controller = new AbortController();
  const client = await SwaplockClient.connect('ws://fixture', {
    expectedChainId: b.CHAIN.chainId,
    signal: controller.signal,
    timeoutMs: 1000,
    createSocket: () => socket,
  });
  t.after(() => client.close());
  const options = {
    transactionId: transactionId(
      b.TransactionCodec.decode(vectors.transfer.transaction),
    ),
    blockNumber: 100,
    transactionIndex: 0,
    maxHeadAgeSeconds: 30,
    maxHeadTimeAheadSeconds: 5,
  };

  return {
    client,
    socket,
    controller,
    options,
  };
}

const inspectionError = (code) => (error) =>
  error instanceof TransactionInspectionError && error.code === code;

test('inspection returns the transaction, signed-header fingerprint and observed irreversibility', async (t) => {
  const state = await fixture(t);
  const result = await state.client.inspectTransactionInBlock(state.options);
  assert.equal(result.transactionId, state.options.transactionId);
  assert.equal(result.blockNumber, 100);
  assert.equal(result.transactionIndex, 0);
  assert.equal(result.observedAt, now);
  assert.equal(result.irreversible, false);
  assert.equal(result.transaction.operations[0][1].amount.amount, 100000n);
  const { transactions, ...header } = state.socket.block;
  const headerBytes = b.encodeMaybeSignedBlockHeader(
    b.MaybeSignedBlockHeaderCodec.decode(header),
  );
  assert.equal(
    result.blockFingerprint,
    createHash('sha256').update(headerBytes).digest('hex'),
  );
  assert.deepEqual(state.socket.reads, [
    'get_dynamic_global_properties',
    'get_block',
    'get_dynamic_global_properties',
  ]);

  state.socket.head.last_irreversible_block_num = 100;
  const irreversible = await state.client.inspectTransactionInBlock(
    state.options,
  );
  assert.equal(irreversible.irreversible, true);
});

for (const absent of ['block', 'position', 'transaction', 'above-head']) {
  test(`${absent} returns absence instead of asserting rejection`, async (t) => {
    const state = await fixture(t);
    if (absent === 'block') {
      state.socket.block = null;
    } else if (absent === 'position') {
      state.options.transactionIndex = 1;
    } else if (absent === 'transaction') {
      state.socket.block.transactions[0].ref_block_num = 999;
    } else {
      state.socket.head.head_block_number = 99;
    }

    assert.equal(
      await state.client.inspectTransactionInBlock(state.options),
      undefined,
    );
  });
}

for (const field of ['head_block_number', 'head_block_id']) {
  test(`a changed ${field} during inspection is unavailable even when the transaction matches`, async (t) => {
    const state = await fixture(t);
    state.socket.onBlock = () => {
      state.socket.head[field] =
        field === 'head_block_number' ? 101 : 'ff'.repeat(20);
    };

    await assert.rejects(
      state.client.inspectTransactionInBlock(state.options),
      inspectionError('head-changed'),
    );
  });
}

for (const phase of ['before', 'after']) {
  for (const offset of [-31, 6]) {
    test(`${phase} head at offset ${offset} violates freshness limits`, async (t) => {
      const state = await fixture(t);
      const changeTime = () => {
        state.socket.head.time = time(offset);
      };
      if (phase === 'before') {
        changeTime();
      } else {
        state.socket.onBlock = changeTime;
      }

      await assert.rejects(
        state.client.inspectTransactionInBlock(state.options),
        inspectionError('invalid-head'),
      );
      if (phase === 'before') {
        assert.deepEqual(state.socket.reads, ['get_dynamic_global_properties']);
      }
    });
  }
}

for (const offset of [-30, 5]) {
  test(`head freshness boundary ${offset} remains inclusive`, async (t) => {
    const state = await fixture(t);
    state.socket.head.time = time(offset);
    assert.ok(await state.client.inspectTransactionInBlock(state.options));
  });
}

test('caller age limit can allow a head older than the SDK default', async (t) => {
  const state = await fixture(t);
  state.socket.head.time = time(-180);
  state.options.maxHeadAgeSeconds = 300;
  assert.ok(await state.client.inspectTransactionInBlock(state.options));
});

test('a replaced signed header changes the fingerprint even for the same transaction', async (t) => {
  const state = await fixture(t);
  const before = await state.client.inspectTransactionInBlock(state.options);
  state.socket.block.witness_signature = '55'.repeat(65);
  const after = await state.client.inspectTransactionInBlock(state.options);
  assert.equal(before.transactionId, after.transactionId);
  assert.notEqual(before.blockFingerprint, after.blockFingerprint);
});

for (const cancelAt of ['before', 'block', 'after']) {
  test(`session cancellation ${cancelAt} the block read rejects without an inclusion result`, async (t) => {
    const state = await fixture(t);
    const reason = new Error('selection ended');
    if (cancelAt === 'before') {
      state.controller.abort(reason);
    } else if (cancelAt === 'block') {
      state.socket.holdBlock = true;
      state.socket.onBlock = () => state.controller.abort(reason);
    } else {
      state.socket.onBlock = () => {
        state.socket.onHead = () => state.controller.abort(reason);
      };
    }

    await assert.rejects(
      state.client.inspectTransactionInBlock(state.options),
      (error) => error === reason,
    );
    assert.equal(
      state.socket.reads.length,
      cancelAt === 'before' ? 0 : cancelAt === 'block' ? 2 : 3,
    );
    assert.equal(getEventListeners(state.socket, 'message').length, 0);
  });
}

for (const [field, value] of [
  ['transactionId', 'invalid'],
  ['blockNumber', 0],
  ['blockNumber', 4294967296],
  ['transactionIndex', -1],
  ['transactionIndex', 0.5],
  ['maxHeadAgeSeconds', -1],
  ['maxHeadTimeAheadSeconds', Infinity],
]) {
  test(`invalid ${field} is rejected before network reads`, async (t) => {
    const state = await fixture(t);
    state.options[field] = value;
    await assert.rejects(state.client.inspectTransactionInBlock(state.options));
    assert.equal(state.socket.reads.length, 0);
  });
}

test('a malformed block response fails instead of reporting absence', async (t) => {
  const state = await fixture(t);
  state.socket.block.transactions[0].ref_block_num = -1;
  await assert.rejects(state.client.inspectTransactionInBlock(state.options));
});

test('inspection keeps the requested identity and position when the caller mutates options during a read', async (t) => {
  const state = await fixture(t);
  const expectedId = state.options.transactionId;
  state.socket.onBlock = () => {
    state.options.transactionId = 'ff'.repeat(20);
    state.options.blockNumber = 200;
    state.options.transactionIndex = 10;
  };
  const result = await state.client.inspectTransactionInBlock(state.options);
  assert.equal(result.transactionId, expectedId);
  assert.equal(result.blockNumber, 100);
  assert.equal(result.transactionIndex, 0);
});
