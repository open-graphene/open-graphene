import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as p from '../graphene-primitives/dist/index.js';
import * as fc from '../graphene-fc/dist/index.js';
import * as c from '../graphene-codec/dist/index.js';
import * as bitshares from '../graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';
import * as actanet from '../graphene-chain-actanet/graphene-chain-actanet-bindings/dist/index.js';

const vectors = JSON.parse(
  readFileSync(new URL('fixtures/protocol-vectors.json', import.meta.url)),
);
test('generic packed IDs and typed varints match shared Rust/native-source vectors', () => {
  for (const v of vectors.objectIds) {
    const id = p.parseObjectId(v.id);
    const generic = new fc.FcWriter();
    generic.objectId(v.id);
    const typed = new fc.FcWriter();
    typed.typedId(v.id, id.space, id.type);
    assert.equal(p.bytesToHex(generic.finish()), v.genericHex);
    assert.equal(p.bytesToHex(typed.finish()), v.typedHex);
  }
  for (const id of vectors.invalidObjectIds)
    assert.throws(() => p.parseObjectId(id));
  assert.throws(() => actanet.AccountId('1.3.1'));
});
test('integer boundaries survive wire JSON and FC without rounding', () => {
  for (const v of vectors.integers) {
    const codec = c.wideInteger(64, v.kind === 'i64');
    const n = codec.decode(c.parseJson(v.decimal));
    assert.equal(n, BigInt(v.decimal));
    assert.equal(c.stringifyJson(codec.encode(n)), v.decimal);
    const writer = new fc.FcWriter();
    v.kind === 'i64' ? writer.int(n, 8) : writer.uint(n, 8);
    assert.equal(p.bytesToHex(writer.finish()), v.hex);
  }
  assert.throws(() => c.wideInteger(64, false).decode(9007199254740992));
  assert.throws(() => c.wideInteger(64, false).decode('18446744073709551616'));
  assert.throws(() => new fc.FcWriter().uint(1, 0));
  assert.throws(() => new fc.FcWriter().int(1, 1.5));
});
for (const [name, chain] of [
  ['bitshares', bitshares],
  ['actanet', actanet],
]) {
  test(`${name}: generated transaction codecs round trip a public transfer`, () => {
    const tx = chain.TransactionCodec.decode(vectors.transfer.transaction);
    assert.equal(tx.operations[0][1].amount.amount, 100000n);
    assert.deepEqual(
      JSON.parse(c.stringifyJson(chain.TransactionCodec.encode(tx))),
      vectors.transfer.transaction,
    );
    assert.deepEqual(
      chain.operation.transfer(tx.operations[0][1]),
      tx.operations[0],
    );
    assert.throws(() => chain.OperationCodec.decode([9999, {}]));
    assert.throws(() =>
      chain.TransactionCodec.decode({
        ...vectors.transfer.transaction,
        expiration: '2026-02-30T00:00:00',
      }),
    );
    assert.throws(() =>
      chain.TransferOperationCodec.encode({ ...tx.operations[0][1], typo: 1 }),
    );
  });
  test(`${name}: RPC descriptors preserve omitted defaults and null object slots`, () => {
    assert.deepEqual(
      chain.DatabaseGetObjects.encodeParams({ ids: ['1.2.1'] }),
      [['1.2.1']],
    );
    assert.deepEqual(
      chain.DatabaseGetObjects.encodeParams({ ids: [], subscribe: null }),
      [[], null],
    );
    const result = chain.DatabaseGetObjects.parseReturns([
      null,
      { id: '254.255.9', new_field: 'future' },
    ]);
    assert.equal(result[0], null);
    assert.equal(result[1].kind, 'unknown');
    assert.equal(result[1].value.new_field, 'future');
    assert.throws(() =>
      chain.DatabaseGetObjects.encodeParams({ ids: [], typo: false }),
    );
    assert.throws(() =>
      chain.DatabaseGetObjects.parseReturns([{ id: '1.2.-1' }]),
    );
  });
}
test('recursive proposal fee responses retain bigint amounts', () => {
  const value = [
    { amount: '9007199254740993', asset_id: '1.3.0' },
    [{ amount: 2, asset_id: '1.3.0' }],
  ];
  const result = actanet.RequiredFeeCodec.decode(value);
  assert.equal(result[0].amount, 9007199254740993n);
  assert.equal(result[1][0].amount, 2n);
  assert.match(
    c.stringifyJson(actanet.RequiredFeeCodec.encode(result)),
    /9007199254740993/,
  );
});
test('actanet: a content card guard extension round trips and rejects unknown guards', () => {
  const operation = {
    fee: { amount: 0, asset_id: '1.3.0' },
    payer: [1, { grant: '1.29.4', sponsor: '1.2.100' }],
    caller: [0, '1.2.101'],
    content_id: '1.27.7',
    new_hash: 'hash',
    extensions: {},
  };
  const codec = actanet.ContentCardUpdateOperationCodec;
  assert.deepEqual(codec.decode(operation).extensions, {});
  const guarded = codec.decode({
    ...operation,
    extensions: { expected_revision: 3 },
  });
  assert.equal(guarded.extensions.expected_revision, 3);
  assert.deepEqual(JSON.parse(c.stringifyJson(codec.encode(guarded))), {
    ...operation,
    extensions: { expected_revision: 3 },
  });
  assert.throws(() =>
    codec.decode({ ...operation, extensions: { misspelled_guard: 3 } }),
  );
  assert.throws(() => codec.decode({ ...operation, extensions: [1] }));
});
test('RPC optional positional holes and nonnullable nulls fail locally', () => {
  const method = c.rpc(
    'database',
    'test',
    [
      { name: 'first', codec: c.text, required: false, nullable: false },
      { name: 'second', codec: c.text, required: false, nullable: false },
    ],
    c.rpcVoid,
  );
  assert.deepEqual(method.encodeParams({}), []);
  assert.throws(() => method.encodeParams({ second: 'x' }));
  assert.throws(() => method.encodeParams({ first: null }));
});
test('digest uses chain ID and raw transaction bytes', () => {
  assert.equal(
    p.bytesToHex(
      fc.transactionDigest(
        vectors.transfer.chainId,
        p.hexToBytes(vectors.transfer.hex),
      ),
    ),
    vectors.transfer.digestHex,
  );
  assert.throws(() => fc.transactionDigest('bad', new Uint8Array()));
});
test('errors locate nested invalid fields', () => {
  const tx = structuredClone(vectors.transfer.transaction);
  tx.operations[0][1].amount.amount = '9223372036854775808';
  assert.throws(
    () => actanet.TransactionCodec.decode(tx),
    (error) => {
      assert.deepEqual(error.path, ['operations', 0, 0, 'amount', 'amount']);
      return true;
    },
  );
});

for (const [name, chain] of [
  ['bitshares', bitshares],
  ['actanet', actanet],
]) {
  test(`${name}: market operation extensions use class-local C++ options objects`, () => {
    const order = {
      fee: { amount: 0, asset_id: '1.3.0' },
      seller: '1.2.100',
      amount_to_sell: { amount: 200, asset_id: '1.3.100' },
      min_to_receive: { amount: 300, asset_id: '1.3.0' },
      expiration: '2026-09-29T16:00:00',
      fill_or_kill: false,
      extensions: {},
    };
    assert.deepEqual(
      JSON.parse(
        c.stringifyJson(
          chain.LimitOrderCreateOperationCodec.encode(
            chain.LimitOrderCreateOperationCodec.decode(order),
          ),
        ),
      ),
      order,
    );
    assert.throws(() =>
      chain.LimitOrderCreateOperationCodec.decode({ ...order, extensions: [] }),
    );
    const margin = {
      fee: order.fee,
      funding_account: '1.2.100',
      delta_collateral: order.min_to_receive,
      delta_debt: order.amount_to_sell,
      extensions: { target_collateral_ratio: 2000 },
    };
    assert.deepEqual(
      JSON.parse(
        c.stringifyJson(
          chain.CallOrderUpdateOperationCodec.encode(
            chain.CallOrderUpdateOperationCodec.decode(margin),
          ),
        ),
      ),
      margin,
    );
  });
}
