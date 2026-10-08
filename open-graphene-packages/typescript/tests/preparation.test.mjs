import test from 'node:test';
import assert from 'node:assert/strict';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import {
  prepareOperations,
  TransactionPreparationError,
} from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import { CodecError } from '../graphene-codec/dist/index.js';

const now = Date.UTC(2026, 9, 8, 12);
const coreFee = {
  amount: '100',
  asset_id: '1.3.0',
};

function fixture(options = {}) {
  const calls = [];
  const builders = b.bindOperationBuilders((operation) => operation);
  const operation = builders.data_room_create({
    owner: b.AccountId('1.2.100'),
    name: 'room',
    description: '',
    subject: [0, {}],
    extensions: {
      write_policy: 1,
    },
  });
  const head = {
    head_block_number: 0x10064,
    head_block_id: Uint8Array.from([
      0,
      1,
      0,
      100,
      17,
      34,
      51,
      68,
      ...new Array(12).fill(0),
    ]),
    time: new Date(now + (options.headOffsetSeconds ?? 0) * 1000)
      .toISOString()
      .slice(0, 19),
  };
  const client = {
    chainId: 'ab'.repeat(32),
    rpc: {
      async invoke(descriptor, parameters) {
        calls.push(descriptor.method);
        await options.before?.(descriptor.method);

        if (descriptor.method === 'get_required_fees') {
          assert.equal(parameters.asset_symbol_or_id, '1.3.0');
          const fees = options.fees ?? [coreFee];
          return descriptor.parseReturns(fees);
        }

        assert.equal(descriptor.method, 'get_dynamic_global_properties');
        return head;
      },
    },
  };

  return {
    client,
    operation,
    head,
    calls,
  };
}

function preparationError(code) {
  return (error) =>
    error instanceof TransactionPreparationError && error.code === code;
}

test('preparation prices a room operation and derives reference block and expiration without mutating the request', async (t) => {
  t.mock.method(Date, 'now', () => now);
  const state = fixture();
  const original = structuredClone(state.operation);
  const prepared = await prepareOperations(state.client, [state.operation], {
    maxFee: 100n,
    expirationSeconds: 120,
    maxHeadAgeSeconds: 30,
    maxHeadTimeAheadSeconds: 5,
  });

  assert.equal(prepared.chainId, state.client.chainId);
  assert.equal(prepared.startBlock, 0x10064);
  assert.equal(prepared.transaction.ref_block_num, 100);
  assert.equal(prepared.transaction.ref_block_prefix, 0x44332211);
  assert.equal(prepared.transaction.expiration, '2026-10-08T12:02:00');
  assert.equal(prepared.transaction.operations[0][1].fee.amount, 100n);
  assert.equal(
    prepared.transaction.operations[0][1].extensions.write_policy,
    1,
  );
  assert.deepEqual(state.operation, original);
  assert.deepEqual(state.calls, [
    'get_required_fees',
    'get_dynamic_global_properties',
  ]);
});

for (const [name, fees] of [
  ['missing fee', []],
  ['extra fee', [coreFee, coreFee]],
  ['wrong asset', [{ amount: '100', asset_id: '1.3.1' }]],
  ['negative fee', [{ amount: '-1', asset_id: '1.3.0' }]],
  ['recursive fee for a non-proposal', [[coreFee, []]]],
]) {
  test(`${name} rejects preparation before reading a reference block`, async () => {
    const state = fixture({
      fees,
    });

    await assert.rejects(
      prepareOperations(state.client, [state.operation]),
      preparationError('invalid-fee'),
    );
    assert.deepEqual(state.calls, ['get_required_fees']);
  });
}

test('fees above the approval have a stable error code and never read a reference block', async () => {
  const state = fixture();

  await assert.rejects(
    prepareOperations(state.client, [state.operation], {
      maxFee: 99n,
    }),
    preparationError('fee-limit'),
  );
  assert.deepEqual(state.calls, ['get_required_fees']);
});

for (const offset of [-31, 6]) {
  test(`a reference block offset by ${offset} seconds violates the caller freshness limits`, async (t) => {
    t.mock.method(Date, 'now', () => now);
    const state = fixture({
      headOffsetSeconds: offset,
    });

    await assert.rejects(
      prepareOperations(state.client, [state.operation], {
        maxHeadAgeSeconds: 30,
        maxHeadTimeAheadSeconds: 5,
      }),
      preparationError('invalid-head'),
    );
  });
}

for (const offset of [-30, 5]) {
  test(`a reference block exactly on the freshness boundary ${offset} is accepted`, async (t) => {
    t.mock.method(Date, 'now', () => now);
    const state = fixture({
      headOffsetSeconds: offset,
    });

    const prepared = await prepareOperations(state.client, [state.operation], {
      maxHeadAgeSeconds: 30,
      maxHeadTimeAheadSeconds: 5,
    });
    assert.equal(prepared.startBlock, state.head.head_block_number);
  });
}

test('a configured age limit can be more permissive than the default', async (t) => {
  t.mock.method(Date, 'now', () => now);
  const state = fixture({
    headOffsetSeconds: -180,
  });

  const prepared = await prepareOperations(state.client, [state.operation], {
    maxHeadAgeSeconds: 300,
    expirationSeconds: 360,
  });
  assert.equal(prepared.transaction.expiration, '2026-10-08T12:03:00');
});

test('reference block zero is rejected even with a current timestamp', async (t) => {
  t.mock.method(Date, 'now', () => now);
  const state = fixture();
  state.head.head_block_number = 0;

  await assert.rejects(
    prepareOperations(state.client, [state.operation]),
    preparationError('invalid-head'),
  );
});

for (const field of ['maxHeadAgeSeconds', 'maxHeadTimeAheadSeconds']) {
  test(`${field} rejects invalid limits before making RPC calls`, async () => {
    const state = fixture();

    for (const value of [-1, 1.5, NaN, Infinity]) {
      await assert.rejects(
        prepareOperations(state.client, [state.operation], {
          [field]: value,
        }),
        /Invalid head freshness limits/,
      );
    }
    assert.deepEqual(state.calls, []);
  });
}

test('preparation captures approval and freshness options before asynchronous RPC work', async (t) => {
  t.mock.method(Date, 'now', () => now);
  const options = {
    maxFee: 100n,
    maxHeadAgeSeconds: 30,
  };
  const state = fixture({
    before() {
      options.maxFee = 0n;
      options.maxHeadAgeSeconds = -1;
    },
  });

  const prepared = await prepareOperations(
    state.client,
    [state.operation],
    options,
  );
  assert.equal(prepared.transaction.operations[0][1].fee.amount, 100n);
});

test('RPC and cancellation failures retain their identity and stop preparation', async () => {
  for (const reason of [
    new Error('transport failed'),
    new CodecError('cancelled'),
    null,
  ]) {
    const state = fixture({
      before() {
        throw reason;
      },
    });

    await assert.rejects(
      prepareOperations(state.client, [state.operation]),
      (error) => error === reason,
    );
    assert.deepEqual(state.calls, ['get_required_fees']);
  }
});
