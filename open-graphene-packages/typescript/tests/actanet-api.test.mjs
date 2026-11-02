import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as b from '../graphene-chain-actanet/graphene-chain-actanet-bindings/dist/index.js';
import {
  ActanetClient,
  ActanetWifSigner,
  BroadcastOutcomeUnknown,
  PreparedTransfer,
} from '../graphene-chain-actanet/graphene-chain-actanet-api/dist/index.js';
import {
  signDigestCompact,
  isCanonicalCompactSignature,
  isLowSCompactSignature,
  recoverPublicKey,
  encodePublicKey,
} from '../graphene-fc/dist/signing.js';
import { hexToBytes, bytesToHex } from '../graphene-primitives/dist/index.js';
import { transactionDigest } from '../graphene-fc/dist/index.js';

const vectors = JSON.parse(
  readFileSync(new URL('fixtures/protocol-vectors.json', import.meta.url)),
);
const key = hexToBytes(vectors.signing.publicTestPrivateKeyHex);
const publicKey = hexToBytes(vectors.signing.publicKeyHex);
const address = encodePublicKey(publicKey, 'ACTA');
const authority = {
  weight_threshold: 1,
  account_auths: [],
  key_auths: [[address, 1]],
  address_auths: [],
};
const time = () => new Date().toISOString().slice(0, 19);
function account(id, name) {
  return {
    id,
    name,
    membership_expiration_date: '1970-01-01T00:00:00',
    registrar: '1.2.0',
    referrer: '1.2.0',
    lifetime_referrer: '1.2.0',
    network_fee_percentage: 0,
    lifetime_referrer_fee_percentage: 0,
    referrer_rewards_percentage: 0,
    owner: authority,
    active: authority,
    options: {
      memo_key: address,
      voting_account: '1.2.0',
      num_witness: 0,
      num_committee: 0,
      votes: [],
      extensions: [],
    },
    num_committee_voted: 0,
    statistics: '2.6.0',
    whitelisting_accounts: [],
    blacklisting_accounts: [],
    whitelisted_accounts: [],
    blacklisted_accounts: [],
    owner_special_authority: [0, {}],
    active_special_authority: [0, {}],
    top_n_control_flags: 0,
    creation_block_num: 1,
    creation_time: time(),
  };
}
class Socket extends EventTarget {
  readyState = 0;
  sent = [];
  submitted = null;
  constructor(options = {}) {
    super();
    this.options = options;
    queueMicrotask(() => {
      this.readyState = 1;
      this.dispatchEvent(new Event('open'));
    });
  }
  send(raw) {
    const m = JSON.parse(raw);
    this.sent.push(m);
    const method = m.params[1],
      args = m.params[2];
    let result;
    switch (method) {
      case 'login':
        result = true;
        break;
      case 'database':
        result = 2;
        break;
      case 'network_broadcast':
        result = 3;
        break;
      case 'get_chain_id':
        result = this.options.chainId ?? b.CHAIN.chainId;
        break;
      case 'get_accounts':
        result = this.options.missing
          ? [null, null]
          : [account('1.2.100', 'alice'), account('1.2.101', 'bob')];
        break;
      case 'get_required_fees':
        result = [{ amount: 100, asset_id: '1.3.0' }];
        break;
      case 'get_account_balances':
        result = [
          { amount: this.options.balance ?? 100000, asset_id: '1.3.0' },
        ];
        break;
      case 'get_dynamic_global_properties':
        result = {
          id: '2.1.0',
          head_block_number: this.submitted ? 101 : 100,
          head_block_id: '00000064' + '11223344' + '00'.repeat(12),
          time: this.options.stale ? '2020-01-01T00:00:00' : time(),
          current_witness: '1.6.1',
          next_maintenance_time: time(),
          last_vote_tally_time: time(),
          last_budget_time: time(),
          witness_budget: 0,
          total_pob: 0,
          total_inactive: 0,
          accounts_registered_this_interval: 0,
          recently_missed_count: 0,
          current_aslot: 1,
          recent_slots_filled: 1,
          dynamic_flags: 0,
          last_irreversible_block_num: 99,
          // rdPoS; FC sends a uint64 this large as a string.
          maintenance_seed: '9007199254740993',
        };
        break;
      case 'broadcast_transaction':
        this.submitted = args[0];
        if (this.options.dropBroadcast) return;
        result = null;
        break;
      case 'get_block':
        result = {
          previous: '00'.repeat(20),
          timestamp: time(),
          witness: '1.6.1',
          transaction_merkle_root: '00'.repeat(20),
          extensions: [],
          witness_signature: '00'.repeat(65),
          transactions: [{ ...this.submitted, operation_results: [] }],
        };
        break;
      default:
        throw Error('Unexpected RPC ' + method);
    }
    queueMicrotask(() =>
      this.dispatchEvent(
        new MessageEvent('message', {
          data: JSON.stringify({ id: m.id, result }),
        }),
      ),
    );
  }
  close() {
    this.readyState = 3;
    this.dispatchEvent(new Event('close'));
  }
}
const request = { from: 'alice', to: 'bob', amount: 1000n, maxFee: 100n };
test('Actanet prepare/sign/broadcast/inclusion uses chain digest, fees and immutable snapshots', async () => {
  let socket;
  const client = await ActanetClient.connect('ws://fixture', {
    createSocket: () => (socket = new Socket()),
  });
  const signer = new ActanetWifSigner(
    '5HpjE2Hs7vjU4SN3YyPQCdhzCu92WoEeuE6PWNuiPyTu3ESGnzn',
  );
  try {
    const prepared = await client.prepareTransfer(request);
    assert.equal(prepared.transaction.ref_block_prefix, 0x44332211);
    assert.equal(prepared.transaction.operations[0][1].fee.amount, 100n);
    const signed = await prepared.sign(signer);
    const signature = signed.transaction.signatures[0];
    assert.ok(isLowSCompactSignature(signature));
    assert.equal(
      bytesToHex(
        recoverPublicKey(
          transactionDigest(b.CHAIN.chainId, prepared.bytes),
          signature,
        ),
      ),
      bytesToHex(publicKey),
    );
    signed.transaction.operations[0][1].amount.amount = 999n;
    assert.equal(signed.transaction.operations[0][1].amount.amount, 1000n);
    assert.equal((await client.broadcast(signed)).transactionId, prepared.id);
    const included = await client.waitForInclusion(signed, 1000);
    assert.equal(included.blockNumber, 101);
    assert.equal(included.transactionId, prepared.id);
    assert.equal(
      socket.sent.filter((m) => m.params[1] === 'broadcast_transaction').length,
      1,
    );
  } finally {
    signer.dispose();
    client.close();
  }
});
test('Actanet refuses wrong chain, missing accounts, stale head, excess fees and insufficient balance', async () => {
  let socket;
  await assert.rejects(
    ActanetClient.connect('ws://fixture', {
      createSocket: () => (socket = new Socket({ chainId: '00'.repeat(32) })),
    }),
    /different chain/,
  );
  assert.equal(socket.readyState, 3);
  for (const [options, req, pattern] of [
    [{}, { ...request, maxFee: 99n }, /exceeds/],
    [{ missing: true }, request, /does not exist/],
    [{ balance: 1099 }, request, /Insufficient/],
    [{ stale: true }, request, /stale/],
    [{}, { ...request, amount: 0n }, /positive bigint/],
  ]) {
    const client = await ActanetClient.connect('ws://fixture', {
      createSocket: () => new Socket(options),
    });
    try {
      await assert.rejects(client.prepareTransfer(req), pattern);
    } finally {
      client.close();
    }
  }
});
test('Actanet lost broadcast response is unknown and is never retried', async () => {
  let socket;
  const c = await ActanetClient.connect('ws://fixture', {
    timeoutMs: 30,
    createSocket: () => (socket = new Socket({ dropBroadcast: true })),
  });
  const signer = new ActanetWifSigner(
    '5HpjE2Hs7vjU4SN3YyPQCdhzCu92WoEeuE6PWNuiPyTu3ESGnzn',
  );
  try {
    const signed = await (await c.prepareTransfer(request)).sign(signer);
    await assert.rejects(
      c.broadcast(signed),
      (e) =>
        e instanceof BroadcastOutcomeUnknown && e.transactionId === signed.id,
    );
    assert.equal(
      socket.sent.filter((m) => m.params[1] === 'broadcast_transaction').length,
      1,
    );
  } finally {
    c.close();
    signer.dispose();
  }
});
const CURVE_ORDER =
  0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141n;
/** The other valid form of a signature: s replaced by n - s, the recovery id's parity flipped. */
function highSTwin(signature) {
  const s = BigInt('0x' + bytesToHex(signature.slice(33)));
  const twin = signature.slice();
  twin.set(hexToBytes((CURVE_ORDER - s).toString(16).padStart(64, '0')), 33);
  twin[0] = 31 + ((signature[0] - 31) ^ 1);
  return twin;
}
test('Actanet accepts low-S signatures without retrying and rejects their high-S twins', async () => {
  let legacyRejected = 0;
  for (let n = 0; n < 32; n++) {
    const digest = new Uint8Array(32);
    digest[31] = n;
    const signature = signDigestCompact(digest, key, 'low-s');
    assert.ok(isLowSCompactSignature(signature));
    assert.ok(!isLowSCompactSignature(highSTwin(signature)));
    if (!isCanonicalCompactSignature(signature)) legacyRejected++;
    assert.equal(
      bytesToHex(recoverPublicKey(digest, signature)),
      bytesToHex(publicKey),
    );
  }
  assert.ok(
    legacyRejected > 0,
    'Must exercise a signature the legacy rule rejects',
  );

  // A transaction whose plain RFC 6979 signature fails the legacy rule, as a deterministic
  // signer such as XKeys would produce it
  let prepared, signature;
  for (let i = 0; i < 64 && !prepared; i++) {
    const tx = b.TransactionCodec.decode({
      ...vectors.transfer.transaction,
      expiration: new Date(Date.now() + 60000 + i * 1000)
        .toISOString()
        .slice(0, 19),
    });
    const candidate = new PreparedTransfer({
      transaction: tx,
      authority: b.AuthorityCodec.decode(authority),
      startBlock: 1,
      chainId: b.CHAIN.chainId,
    });
    const digest = transactionDigest(b.CHAIN.chainId, candidate.bytes);
    const plain = signDigestCompact(digest, key, 'low-s');
    if (!isCanonicalCompactSignature(plain)) {
      prepared = candidate;
      signature = plain;
    }
  }
  assert.ok(prepared, 'Must find a signature the legacy rule rejects');
  await assert.rejects(
    prepared.sign({ publicKey, signDigest: async () => highSTwin(signature) }),
    /non-canonical/,
  );
  const signed = await prepared.sign({
    publicKey,
    signDigest: async () => signature,
  });
  assert.deepEqual(signed.transaction.signatures[0], signature);
  const malformed = new Uint8Array(65);
  malformed[0] = 31;
  malformed[1] = 1;
  malformed[33] = 1;
  await assert.rejects(
    prepared.sign({ publicKey, signDigest: async () => malformed }),
    /invalid signature/,
  );
});
test('Actanet spec pins the ACTA prefix and the chain ID computed from its genesis', () => {
  assert.equal(b.CHAIN.id, 'actanet');
  assert.equal(b.CHAIN.publicKeyPrefix, 'ACTA');
  assert.match(b.CHAIN.chainId, /^[0-9a-f]{64}$/);
});
