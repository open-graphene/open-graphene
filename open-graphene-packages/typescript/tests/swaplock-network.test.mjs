import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import {
  SwaplockClient,
  PreparedTransfer,
  PreparedTransaction,
} from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import { Graphene } from '../graphene/dist/index.js';
import {
  encodePublicKey,
  encodeWif,
  signDigestCompact,
  verifyDigestCompact,
} from '../graphene-fc/dist/signing.js';

const vectors = JSON.parse(
  readFileSync(new URL('fixtures/protocol-vectors.json', import.meta.url)),
);
const privateKey = Buffer.from(vectors.signing.publicTestPrivateKeyHex, 'hex');
const publicKey = Buffer.from(vectors.signing.publicKeyHex, 'hex');
const address = encodePublicKey(publicKey, b.CHAIN.publicKeyPrefix);
const alternateChainId = 'ab'.repeat(32);
const authority = {
  weight_threshold: 1,
  account_auths: [],
  key_auths: [[address, 1]],
  address_auths: [],
};
const request = {
  from: 'alice',
  to: 'bob',
  amount: 1000n,
  maxFee: 100n,
};

function time() {
  return new Date().toISOString().slice(0, 19);
}

function account(id, name) {
  return {
    id,
    name,
    membership_expiration_date: '2100-01-01T00:00:00',
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
  methods = [];

  constructor(chainId) {
    super();
    this.chainId = chainId;

    queueMicrotask(() => {
      this.readyState = 1;
      this.dispatchEvent(new Event('open'));
    });
  }

  send(raw) {
    const message = JSON.parse(raw);
    const method = message.params[1];
    const args = message.params[2];
    this.methods.push(method);
    let result;

    switch (method) {
      case 'login':
        result = true;
        break;
      case 'database':
        result = 2;
        break;
      case 'get_chain_id':
        result = this.chainId;
        break;
      case 'get_accounts':
        result = [account('1.2.100', 'alice'), account('1.2.101', 'bob')];
        break;
      case 'get_required_fees':
        result = args[0].map(() => ({
          amount: 100,
          asset_id: '1.3.0',
        }));
        break;
      case 'get_account_balances':
        result = [
          {
            amount: 100000,
            asset_id: '1.3.0',
          },
        ];
        break;
      case 'get_dynamic_global_properties':
        result = {
          id: '2.1.0',
          head_block_number: 100,
          head_block_id: '00000064' + '11223344' + '00'.repeat(12),
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
        break;
      default:
        throw new Error(`Unexpected RPC ${method}`);
    }

    queueMicrotask(() => {
      this.dispatchEvent(
        new MessageEvent('message', {
          data: JSON.stringify({
            id: message.id,
            result,
          }),
        }),
      );
    });
  }

  close() {
    this.readyState = 3;
    this.dispatchEvent(new Event('close'));
  }
}

function digestFor(chainId, bytes) {
  return createHash('sha256')
    .update(Buffer.from(chainId, 'hex'))
    .update(bytes)
    .digest();
}

function verifyNetworkSignature(prepared, signed, chainId) {
  const signature = signed.transaction.signatures[0];
  const digest = digestFor(chainId, prepared.bytes);
  const otherChainId =
    chainId === alternateChainId ? b.CHAIN.chainId : alternateChainId;
  const otherDigest = digestFor(otherChainId, prepared.bytes);

  assert.equal(verifyDigestCompact(digest, signature, publicKey), true);
  assert.equal(verifyDigestCompact(otherDigest, signature, publicKey), false);
}

test('Swaplock connections and probes require a valid explicit network before opening sockets', async () => {
  let opened = 0;
  const createSocket = () => {
    opened += 1;
    return new Socket(alternateChainId);
  };
  const invalidIds = [
    undefined,
    null,
    '',
    'ab',
    'AB'.repeat(32),
    'gg'.repeat(32),
  ];

  for (const expectedChainId of invalidIds) {
    const options = {
      expectedChainId,
      createSocket,
    };
    await assert.rejects(
      SwaplockClient.connect('ws://fixture', options),
      /chain ID/,
    );
    await assert.rejects(
      Graphene.swaplock('ws://fixture', options),
      /chain ID/,
    );
    await assert.rejects(
      async () => Graphene.connect('swaplock', 'ws://fixture', options),
      /chain ID/,
    );
    assert.throws(
      () => SwaplockClient.probeLatencies(['ws://fixture'], options),
      /chain ID/,
    );
  }

  await assert.rejects(SwaplockClient.connect('ws://fixture'), /chain ID/);
  assert.equal(opened, 0);
});

for (const strategy of ['first-available', 'lowest-latency']) {
  test(`${strategy} selects only nodes of the configured network and closes rejected sockets`, async (t) => {
    const sockets = [];
    const client = await Graphene.connect(
      'swaplock',
      ['ws://wrong', 'ws://right'],
      {
        expectedChainId: alternateChainId,
        strategy,
        createSocket(endpoint) {
          const socket = new Socket(
            endpoint === 'ws://right' ? alternateChainId : b.CHAIN.chainId,
          );
          sockets.push(socket);
          return socket;
        },
      },
    );
    t.after(() => client.close());

    assert.equal(client.chainId, alternateChainId);
    assert.equal(client.rpc.endpoint, 'ws://right');
    assert.ok(
      sockets
        .filter((socket) => socket.chainId !== alternateChainId)
        .every((socket) => socket.readyState === 3),
    );
  });
}

test('a mismatched network rejects connection before any transaction RPC and closes the socket', async () => {
  const socket = new Socket(b.CHAIN.chainId);
  await assert.rejects(
    SwaplockClient.connect('ws://wrong', {
      expectedChainId: alternateChainId,
      createSocket: () => socket,
    }),
    /different chain ID/,
  );

  assert.equal(socket.readyState, 3);
  assert.deepEqual(socket.methods, ['login', 'database', 'get_chain_id']);
});

test('latency probes filter mismatched networks and close every probe socket', async () => {
  const sockets = [];
  const results = await SwaplockClient.probeLatencies(
    ['ws://wrong', 'ws://right'],
    {
      expectedChainId: alternateChainId,
      createSocket(endpoint) {
        const socket = new Socket(
          endpoint === 'ws://right' ? alternateChainId : b.CHAIN.chainId,
        );
        sockets.push(socket);
        return socket;
      },
    },
  );

  assert.deepEqual(
    results.map((result) => result.chainId),
    [alternateChainId],
  );
  assert.ok(sockets.every((socket) => socket.readyState === 3));
});

test('reconnect preserves the original network after caller options change', async (t) => {
  const sockets = [];
  let nodeChainId = alternateChainId;
  const options = {
    expectedChainId: alternateChainId,
    createSocket() {
      const socket = new Socket(nodeChainId);
      sockets.push(socket);
      return socket;
    },
  };
  const client = await SwaplockClient.connect('ws://fixture', options);
  t.after(() => client.close());
  const prepared = await client.prepareTransfer(request);
  options.expectedChainId = b.CHAIN.chainId;
  nodeChainId = b.CHAIN.chainId;

  await assert.rejects(client.reconnect(), /different chain ID/);
  assert.equal(sockets[1].readyState, 3);
  assert.equal(client.chainId, alternateChainId);

  nodeChainId = alternateChainId;
  await client.reconnect();
  assert.equal(sockets[0].readyState, 3);
  assert.equal(client.chainId, alternateChainId);
  assert.equal(prepared.chainId, alternateChainId);
  const signed = await prepared.sign({
    publicKey,
    signDigest: async (digest) => signDigestCompact(digest, privateKey),
  });
  verifyNetworkSignature(prepared, signed, alternateChainId);
});

for (const chainId of [b.CHAIN.chainId, alternateChainId]) {
  test(`transfer and generic builders sign the configured network ${chainId.slice(0, 8)}`, async (t) => {
    const socket = new Socket(chainId);
    const client = await Graphene.swaplock('ws://fixture', {
      expectedChainId: chainId,
      createSocket: () => socket,
    });
    t.after(() => client.close());
    const transfer = await client.prepareTransfer(request);
    const generic = await client.prepareOperations(
      transfer.transaction.operations,
      {
        maxFee: 100n,
      },
    );
    const builder = await client.operations
      .transfer({
        from: b.AccountId('1.2.100'),
        to: b.AccountId('1.2.101'),
        amount: {
          amount: 1000n,
          asset_id: b.AssetId('1.3.0'),
        },
      })
      .maxFee(100n)
      .prepare();
    client.close();

    for (const prepared of [transfer, generic, builder]) {
      assert.equal(prepared.chainId, chainId);
      assert.throws(() => {
        prepared.chainId = '00'.repeat(32);
      }, TypeError);
      const signed = await prepared.sign({
        publicKey,
        async signDigest(digest) {
          assert.deepEqual(
            Buffer.from(digest),
            digestFor(chainId, prepared.bytes),
          );
          return signDigestCompact(digest, privateKey);
        },
      });
      verifyNetworkSignature(prepared, signed, chainId);

      await assert.rejects(
        prepared.sign({
          publicKey,
          signDigest: async () =>
            signDigestCompact(
              digestFor('cd'.repeat(32), prepared.bytes),
              privateKey,
            ),
        }),
        /invalid signature/,
      );
    }

    const signedWithWif = await generic.signWithWifs([
      {
        wif: encodeWif(privateKey),
        expectedPublicKey: address,
      },
    ]);
    verifyNetworkSignature(generic, signedWithWif, chainId);
    assert.equal(
      socket.methods.some((method) => method.startsWith('broadcast_')),
      false,
    );
  });
}

for (const Prepared of [PreparedTransfer, PreparedTransaction]) {
  test(`${Prepared.name} validates and captures constructor network and transaction values`, async () => {
    const transaction = b.TransactionCodec.decode({
      ...vectors.transfer.transaction,
      expiration: new Date(Date.now() + 60000).toISOString().slice(0, 19),
    });
    const options = {
      transaction,
      authority,
      startBlock: 100,
      chainId: alternateChainId,
    };
    for (const chainId of [undefined, '', 'bad', 'AB'.repeat(32)]) {
      assert.throws(
        () =>
          new Prepared({
            ...options,
            chainId,
          }),
        /chain ID/,
      );
    }

    const prepared = new Prepared(options);
    const originalBytes = prepared.bytes;
    options.chainId = b.CHAIN.chainId;
    transaction.ref_block_num = 999;
    prepared.transaction.ref_block_num = 999;

    assert.equal(prepared.chainId, alternateChainId);
    assert.deepEqual(prepared.bytes, originalBytes);
    const signed = await prepared.sign({
      publicKey,
      signDigest: async (digest) => signDigestCompact(digest, privateKey),
    });
    verifyNetworkSignature(prepared, signed, alternateChainId);
  });
}
