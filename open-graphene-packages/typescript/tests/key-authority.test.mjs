import test from 'node:test';
import assert from 'node:assert/strict';
import {
  canKeySatisfyAuthority,
  canKeySignAlone,
} from '../graphene-core/dist/index.js';

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

const company = 'company-account';
const mateusz = '1.2.254';
const ania = '1.2.255';
const deeper = '1.2.256';

/** Authorities by account ID, read in the order asked. */
function chainOf(authorities) {
  const reads = [];
  return {
    reads,
    activeAuthorities: async (accountIds) => {
      reads.push([...accountIds]);
      return accountIds.map((id) => authorities[id]);
    },
  };
}

function authorityOf(threshold, keys, accounts = []) {
  return {
    weight_threshold: threshold,
    key_auths: keys,
    account_auths: accounts,
  };
}

test('a delegate account with enough weight signs alone for the account', async () => {
  const chain = chainOf({
    [mateusz]: authorityOf(1, [[key, 1]]),
    [ania]: authorityOf(1, [[other, 1]]),
  });

  const signs = await canKeySignAlone({
    authority: authorityOf(
      50,
      [],
      [
        [mateusz, 80],
        [ania, 20],
      ],
    ),
    publicKey: key,
    activeAuthorities: chain.activeAuthorities,
  });

  assert.equal(signs, true);
  // Only the delegate with enough weight is read.
  assert.deepEqual(chain.reads, [[mateusz]]);
  const aniaSigns = await canKeySignAlone({
    authority: authorityOf(
      50,
      [],
      [
        [mateusz, 80],
        [ania, 20],
      ],
    ),
    publicKey: other,
    activeAuthorities: chain.activeAuthorities,
  });
  assert.equal(aniaSigns, false);
});

test('account authorities are followed at most two accounts deep', async () => {
  const chain = chainOf({
    [mateusz]: authorityOf(1, [], [[ania, 1]]),
    [ania]: authorityOf(1, [], [[deeper, 1]]),
    [deeper]: authorityOf(1, [[key, 1]]),
  });

  const signs = await canKeySignAlone({
    authority: authorityOf(1, [], [[mateusz, 1]]),
    publicKey: key,
    activeAuthorities: chain.activeAuthorities,
  });

  assert.equal(signs, false);
  assert.deepEqual(chain.reads, [[mateusz], [ania]]);
});

test('a directly listed key signs without reading delegates', async () => {
  const chain = chainOf({});

  const signs = await canKeySignAlone({
    authority: authorityOf(1, [[key, 1]], [[mateusz, 1]]),
    publicKey: key,
    activeAuthorities: chain.activeAuthorities,
  });

  assert.equal(signs, true);
  assert.deepEqual(chain.reads, []);
});
