import test from 'node:test';
import assert from 'node:assert/strict';
import {
  MAX_OBJECT_ID_INSTANCE,
  objectId,
  parseObjectId,
  typedId,
} from '../graphene-primitives/dist/index.js';
import { objectId as objectIdCodec } from '../graphene-codec/dist/index.js';
import { FcWriter } from '../graphene-fc/dist/index.js';
import * as swaplock from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import * as bitshares from '../graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';

test('canonical object IDs preserve zero and the largest packed ID', () => {
  assert.deepEqual(parseObjectId('0.0.0'), {
    space: 0,
    type: 0,
    instance: 0n,
  });
  assert.deepEqual(parseObjectId('255.255.281474976710655'), {
    space: 255,
    type: 255,
    instance: MAX_OBJECT_ID_INSTANCE,
  });

  const writer = new FcWriter();
  writer.objectId('255.255.281474976710655');

  assert.deepEqual(writer.finish(), new Uint8Array(8).fill(255));
  assert.equal(objectId('1.2.0'), '1.2.0');
  assert.equal(typedId('1.24.281474976710655', 1, 24), '1.24.281474976710655');
});

for (const value of [
  '01.2.3',
  '1.02.3',
  '1.2.003',
  '1.2.00',
  '256.2.3',
  '1.256.3',
  '1.2.281474976710656',
  '1.2.-1',
  '1.2.+1',
  '1.2.1.0',
  '1.2.1e3',
  ' 1.2.3',
  '1.2.3 ',
  '1.2.3\n',
  '1.2.3\r\n',
  '1.2.',
]) {
  test(`rejects noncanonical or out-of-range ID ${JSON.stringify(value)} at public encoding boundaries`, () => {
    assert.throws(() => parseObjectId(value));
    assert.throws(() => objectId(value));
    assert.throws(() => typedId(value, 1, 2));
    assert.throws(() => new FcWriter().objectId(value));
    assert.throws(() => objectIdCodec().encode(value));
    assert.throws(() => objectIdCodec().decode(value));

    for (const chain of [swaplock, bitshares]) {
      assert.throws(() => chain.AccountId(value));
      assert.throws(() =>
        chain.DatabaseGetObjects.encodeParams({
          ids: [value],
          subscribe: false,
        }),
      );
    }
  });
}

test('rejects non-string and oversized object IDs', () => {
  for (const value of [
    undefined,
    null,
    123,
    1n,
    {},
    ['1.2.3'],
    '1.2.' + '9'.repeat(10000),
  ]) {
    assert.throws(() => parseObjectId(value));
  }
});

test('enforces the requested space and type without rejecting instance zero', () => {
  assert.equal(parseObjectId('1.24.0', 1, 24).instance, 0n);
  assert.throws(() => parseObjectId('2.24.0', 1, 24));
  assert.throws(() => parseObjectId('1.23.0', 1, 24));

  for (const chain of [swaplock, bitshares]) {
    assert.equal(chain.AccountId('1.2.0'), '1.2.0');
    assert.throws(() => chain.AccountId('1.3.0'));
  }
});
