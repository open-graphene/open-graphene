import test from 'node:test';
import assert from 'node:assert/strict';
import { canKeySatisfyAuthority } from '../graphene-core/dist/index.js';

const key = 'selected-key';
const other = 'other-key';

for (const [name, threshold, keys, expected] of [
  ['exact threshold', 2, [[key, 2]], true],
  ['above threshold', 2, [[key, 3]], true],
  ['insufficient weight', 2, [[key, 1]], false],
  ['missing key', 1, [[other, 5]], false],
  ['no keys', 1, [], false],
  ['zero threshold', 0, [[key, 1]], false],
  ['zero weight', 1, [[key, 0]], false],
  [
    'duplicate selected key',
    2,
    [
      [key, 2],
      [key, 2],
    ],
    false,
  ],
  [
    'duplicate weights do not add up',
    2,
    [
      [key, 1],
      [key, 1],
    ],
    false,
  ],
  [
    'another key cannot contribute',
    2,
    [
      [key, 1],
      [other, 10],
    ],
    false,
  ],
  [
    'another key does not prevent independent authority',
    2,
    [
      [key, 2],
      [other, 10],
    ],
    true,
  ],
  ['maximum key weight', 65535, [[key, 65535]], true],
  ['threshold beyond one key capacity', 65536, [[key, 65535]], false],
]) {
  test(name, () => {
    assert.equal(
      canKeySatisfyAuthority(
        {
          weight_threshold: threshold,
          key_auths: keys,
        },
        key,
      ),
      expected,
    );
  });
}

for (const threshold of [-1, 0.5, NaN, Infinity, 4294967296]) {
  test(`invalid threshold ${threshold} cannot authorize a key`, () => {
    assert.equal(
      canKeySatisfyAuthority(
        {
          weight_threshold: threshold,
          key_auths: [[key, 1]],
        },
        key,
      ),
      false,
    );
  });
}

for (const weight of [-1, 1.5, NaN, Infinity, 65536]) {
  test(`invalid weight ${weight} cannot authorize a key`, () => {
    assert.equal(
      canKeySatisfyAuthority(
        {
          weight_threshold: 1,
          key_auths: [[key, weight]],
        },
        key,
      ),
      false,
    );
  });
}

test('delegated accounts and addresses cannot supply the missing direct key weight', () => {
  const authority = {
    weight_threshold: 2,
    key_auths: [[key, 1]],
    account_auths: [['1.2.10', 2]],
    address_auths: [['address', 2]],
  };
  assert.equal(canKeySatisfyAuthority(authority, key), false);
  authority.key_auths[0][1] = 2;
  assert.equal(canKeySatisfyAuthority(authority, key), true);
});
