import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as bitshares from '../graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';
import * as actanet from '../graphene-chain-actanet/graphene-chain-actanet-bindings/dist/index.js';
import {
  PreparedTransaction,
  prepareOperations,
} from '../graphene-chain-actanet/graphene-chain-actanet-api/dist/index.js';
import { encryptMemo, decryptMemo } from '../graphene-fc/dist/memo.js';
import { decodePublicKey } from '../graphene-fc/dist/index.js';
import { bytesToHex, hexToBytes } from '../graphene-primitives/dist/index.js';
import {
  WifSigner,
  encodeWif,
  privateKeyFromSeed,
  encodePublicKey,
} from '../graphene-fc/dist/signing.js';
import {
  decimalToRawAmount,
  formatRawAmount,
  analyzeAuthority,
  isAccountName,
} from '../graphene-core/dist/index.js';
const fixture = (name) =>
  JSON.parse(readFileSync(new URL('fixtures/' + name, import.meta.url)));
// Actanet has no public node yet: its vectors were confirmed on a local actanet-core node
// (fc-parity-vectors.mjs --live --node=...). What it keeps from BitShares is also checked against
// the BitShares vectors below, which native C++ confirmed.
const nativeChecked = new Set(['bitshares', 'actanet']);
for (const [chain, b] of [
  ['bitshares', bitshares],
  ['actanet', actanet],
]) {
  test(
    chain +
      ': every nonvirtual operation matches independent Rust' +
      (nativeChecked.has(chain) ? ' and native C++' : '') +
      ' vectors',
    () => {
      const { vectors } = fixture(chain + '-fc-parity.json');
      assert.equal(
        vectors.length,
        Object.keys(b.bindOperationBuilders((op) => op)).length,
      );
      for (const v of [
        ...vectors,
        ...fixture(chain + '-fc-parity-rich.json').vectors,
      ]) {
        assert.equal(
          bytesToHex(b.encodeOperation(b.OperationCodec.decode(v.operation))),
          v.rust.hex,
          v.name,
        );
        if (nativeChecked.has(chain)) assert.equal(v.nativeMatch, true, v.name);
      }
      for (const tag of b.virtualOperationTags) {
        assert.throws(() => b.encodeOperation([tag, {}])); // Payload/schema validation must never permit signing a virtual op.
      }
    },
  );
}
// What Actanet changes in the BitShares 7.0.4 operations; everything else stays byte for byte.
const actanetChanges = {
  // rdPoS adds witness pool sizes to chain_parameters.
  changed: ['committee_member_update_global_parameters_operation'],
  added: [
    'commit_create_operation',
    'reveal_create_operation',
    // Data rooms, content cards, content card grants and fee grants
    'data_room_create_operation',
    'data_room_update_operation',
    'data_room_delete_operation',
    'data_room_member_add_operation',
    'data_room_member_update_operation',
    'data_room_member_remove_operation',
    'data_room_rotate_key_operation',
    'content_card_create_operation',
    'content_card_update_operation',
    'content_card_remove_operation',
    'content_card_grant_create_operation',
    'content_card_grant_revoke_operation',
    'fee_grant_create_operation',
    'fee_grant_update_operation',
    'fee_grant_revoke_operation',
  ],
};
// The key prefix lives only in the JSON form; the FC bytes of a public key do not carry it.
const reprefixKeys = (value, from, to) => {
  if (typeof value === 'string' && value.startsWith(from)) {
    try {
      return encodePublicKey(decodePublicKey(value, from), to);
    } catch {
      return value;
    }
  }
  if (Array.isArray(value)) return value.map((v) => reprefixKeys(v, from, to));
  if (value && typeof value === 'object')
    return Object.fromEntries(
      Object.entries(value).map(([k, v]) => [k, reprefixKeys(v, from, to)]),
    );
  return value;
};
test('actanet: every operation kept from BitShares matches the native-checked BitShares vectors', () => {
  const names = (chain) =>
    fixture(chain + '-fc-parity.json').vectors.map((v) => v.name);
  const own = names('actanet'),
    upstream = names('bitshares');
  assert.deepEqual(
    own.filter((n) => !upstream.includes(n)),
    actanetChanges.added,
  );
  assert.deepEqual(
    upstream.filter((n) => !own.includes(n)),
    [],
  );
  for (const v of [
    ...fixture('bitshares-fc-parity.json').vectors,
    ...fixture('bitshares-fc-parity-rich.json').vectors,
  ]) {
    if (actanetChanges.changed.includes(v.name)) continue;
    const operation = reprefixKeys(
      v.operation,
      bitshares.CHAIN.publicKeyPrefix,
      actanet.CHAIN.publicKeyPrefix,
    );
    assert.equal(
      bytesToHex(
        actanet.encodeOperation(actanet.OperationCodec.decode(operation)),
      ),
      v.rust.hex,
      v.name,
    );
    assert.equal(v.nativeMatch, true, v.name);
  }
});
test('memo ciphertext matches Rust; peer decrypts, wrong nonce/key and tampering fail', async () => {
  const v = fixture('memo-rust.json'),
    alice = hexToBytes(v.alicePrivateKeyHex),
    bob = hexToBytes(v.bobPrivateKeyHex),
    ap = hexToBytes(v.alicePublicKeyHex),
    bp = hexToBytes(v.bobPublicKeyHex);
  const nonce = BigInt(v.nonce),
    message = new TextEncoder().encode(v.message);
  const cipher = await encryptMemo(alice, bp, nonce, message);
  assert.equal(bytesToHex(cipher), v.ciphertextHex);
  assert.deepEqual(await decryptMemo(bob, ap, nonce, cipher), message);
  await assert.rejects(
    decryptMemo(bob, ap, nonce + 1n, cipher),
    /decryption failed/,
  );
  const bad = cipher.slice();
  bad[0] ^= 1;
  await assert.rejects(decryptMemo(bob, ap, nonce, bad), /decryption failed/);
  await assert.rejects(
    decryptMemo(alice, ap, nonce, cipher),
    /decryption failed/,
  );
  const huge = 9007199254740993n;
  assert.deepEqual(
    await decryptMemo(
      bob,
      ap,
      huge,
      await encryptMemo(alice, bp, huge, new Uint8Array()),
    ),
    new Uint8Array(),
  );
});
test('generic transactions collect distinct signatures without changing transaction ID', async () => {
  const b = actanet,
    v = fixture('protocol-vectors.json').transfer.transaction;
  const tx = b.TransactionCodec.decode({
    ...v,
    expiration: new Date(Date.now() + 60000).toISOString().slice(0, 19),
  });
  const prepared = new PreparedTransaction({
    transaction: tx,
    startBlock: 10,
    chainId: b.CHAIN.chainId,
  });
  const alice = new WifSigner(
    encodeWif(privateKeyFromSeed(new TextEncoder().encode('alice'))),
  );
  const bob = new WifSigner(
    encodeWif(privateKeyFromSeed(new TextEncoder().encode('bob'))),
  );
  try {
    const signed = await prepared.sign([alice, bob, alice]);
    assert.equal(signed.transaction.signatures.length, 2);
    assert.equal((await prepared.sign(alice)).id, signed.id);
    await assert.rejects(prepared.sign([]), /No signing keys/);
    await assert.rejects(
      prepared.signWithWifs([
        {
          wif: encodeWif(privateKeyFromSeed(new TextEncoder().encode('alice'))),
          expectedPublicKey: encodePublicKey(
            bob.publicKey,
            b.CHAIN.publicKeyPrefix,
          ),
        },
      ]),
      /differs/,
    );
  } finally {
    alice.dispose();
    bob.dispose();
  }
});
test('FC containers enforce protocol ordering and reject unsupported populated extensions', () => {
  const b = actanet,
    asset = { amount: 0n, asset_id: b.AssetId('1.3.0') };
  const authority = {
    weight_threshold: 1,
    account_auths: [
      [b.AccountId('1.2.10'), 1],
      [b.AccountId('1.2.2'), 1],
    ],
    key_auths: [],
    address_auths: [],
  };
  assert.throws(() => b.encodeAuthority(authority), /sorted and unique/);
  assert.doesNotThrow(() =>
    b.encodeAuthority({
      ...authority,
      account_auths: [...authority.account_auths].reverse(),
    }),
  );
  const order = b
    .bindOperationBuilders((op) => op)
    .limit_order_create({
      seller: b.AccountId('1.2.1'),
      amount_to_sell: asset,
      min_to_receive: asset,
      expiration: '2026-09-29T00:00:00',
      fill_or_kill: false,
    });
  assert.doesNotThrow(() => b.encodeOperation(order));
  assert.throws(
    () =>
      b.encodeOperation([
        order[0],
        { ...order[1], extensions: { on_fill: [] } },
      ]),
    /Nonempty extension/,
  );
});
test('amounts, account validation and authority analysis preserve Rust semantics', () => {
  assert.equal(decimalToRawAmount('90071992547.40993', 5), 9007199254740993n);
  assert.equal(decimalToRawAmount('1.230000', 2), 123n);
  assert.equal(formatRawAmount(-123n, 5), '-0.00123');
  assert.throws(() => decimalToRawAmount('0.001', 2), /decimal places/);
  assert.equal(isAccountName('alice-bob.test'), true);
  assert.equal(isAccountName('al--ice'), false);
  const analysis = analyzeAuthority(2, [
    { id: 'a', weight: 1 },
    { id: 'b', weight: 1 },
    { id: 'c', weight: 1 },
  ]);
  assert.deepEqual(analysis.minimalSignerSets, [
    ['a', 'b'],
    ['a', 'c'],
    ['b', 'c'],
  ]);
  assert.equal(analyzeAuthority(2, [{ id: 'a', weight: 1 }]).reachable, false);
  assert.equal(
    analyzeAuthority(1, [
      { id: 'a', weight: 1 },
      { id: 'a', weight: 1 },
    ]).reachable,
    false,
  );
});
test('recursive proposal fees are written to nested operations and included in fee cap', async () => {
  const b = actanet,
    transfer = b.OperationCodec.decode(
      fixture('protocol-vectors.json').transfer.transaction.operations[0],
    );
  const proposal = b
    .bindOperationBuilders((op) => op)
    .proposal_create({
      fee_paying_account: b.AccountId('1.2.1'),
      expiration_time: '2026-09-30T00:00:00',
      proposed_ops: [{ op: transfer }],
    });
  const fee = (n) => ({ amount: n, asset_id: b.AssetId('1.3.0') });
  const client = {
    chainId: b.CHAIN.chainId,
    rpc: {
      invoke: async (d) =>
        d.method === 'get_required_fees'
          ? [[fee(100n), [fee(200n)]]]
          : {
              head_block_number: 1,
              head_block_id: new Uint8Array(20),
              time: new Date().toISOString().slice(0, 19),
            },
    },
  };
  const prepared = await prepareOperations(client, [proposal], {
    maxFee: 300n,
  });
  assert.equal(prepared.transaction.operations[0][1].fee.amount, 100n);
  assert.equal(
    prepared.transaction.operations[0][1].proposed_ops[0].op[1].fee.amount,
    200n,
  );
  await assert.rejects(
    prepareOperations(client, [proposal], { maxFee: 299n }),
    /maximum fee/,
  );
});
