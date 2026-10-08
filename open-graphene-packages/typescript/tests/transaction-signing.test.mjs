import test from 'node:test';
import assert from 'node:assert/strict';
import { getEventListeners } from 'node:events';
import { readFileSync } from 'node:fs';
import { setImmediate as nextTurn } from 'node:timers/promises';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import {
  PreparedTransaction,
  TransactionSigningError,
} from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import {
  signDigestCompact,
  publicKeyFromPrivateKey,
  verifyDigestCompact,
  encodeWif,
} from '../graphene-fc/dist/signing.js';
import { transactionDigest } from '../graphene-fc/dist/index.js';

const vectors = JSON.parse(
  readFileSync(new URL('fixtures/protocol-vectors.json', import.meta.url)),
);
const privateKey = Buffer.from(vectors.signing.publicTestPrivateKeyHex, 'hex');
const publicKey = Buffer.from(vectors.signing.publicKeyHex, 'hex');

function prepare(expiration = Date.now() + 60000) {
  const transaction = b.TransactionCodec.decode({
    ...vectors.transfer.transaction,
    expiration: new Date(expiration).toISOString().slice(0, 19),
  });
  return new PreparedTransaction({
    transaction,
    chainId: 'ab'.repeat(32),
    startBlock: 100,
  });
}

function signingError(code) {
  return (error) =>
    error instanceof TransactionSigningError && error.code === code;
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((onResolve, onReject) => {
    resolve = onResolve;
    reject = onReject;
  });
  return {
    promise,
    resolve,
    reject,
  };
}

test('expired transactions and absent signers have stable error codes', async () => {
  let calls = 0;
  const signer = {
    publicKey,
    async signDigest() {
      calls += 1;
      return new Uint8Array(65);
    },
  };

  await assert.rejects(
    prepare(Date.now() - 1000).sign(signer),
    signingError('expired'),
  );
  await assert.rejects(prepare().sign([]), signingError('no-signers'));
  assert.equal(calls, 0);
});

for (const signature of [new Uint8Array(65), new Uint8Array(0), null]) {
  test(`malformed signature ${signature?.length ?? 'null'} has a stable error code`, async () => {
    await assert.rejects(
      prepare().sign({
        publicKey,
        signDigest: async () => signature,
      }),
      signingError('invalid-signature'),
    );
  });
}

test('wrong-network signatures and signer digest mutation are rejected', async () => {
  const prepared = prepare();
  const wrongDigest = transactionDigest(b.CHAIN.chainId, prepared.bytes);

  await assert.rejects(
    prepared.sign({
      publicKey,
      signDigest: async () => signDigestCompact(wrongDigest, privateKey),
    }),
    signingError('invalid-signature'),
  );
  await assert.rejects(
    prepared.sign({
      publicKey,
      async signDigest(digest) {
        digest.fill(0);
        return signDigestCompact(digest, privateKey);
      },
    }),
    signingError('invalid-signature'),
  );
});

test('pre-aborted signing never invokes the external signer or initializes WIF keys', async () => {
  const controller = new AbortController();
  const reason = new Error('selection ended');
  controller.abort(reason);
  let calls = 0;
  const prepared = prepare();

  await assert.rejects(
    prepared.sign(
      {
        publicKey,
        async signDigest() {
          calls += 1;
          return new Uint8Array(65);
        },
      },
      {
        signal: controller.signal,
      },
    ),
    (error) => error === reason,
  );
  await assert.rejects(
    prepared.signWithWifs(
      [
        {
          wif: 'invalid WIF must not be read',
          expectedPublicKey: 'invalid',
        },
      ],
      {
        signal: controller.signal,
      },
    ),
    (error) => error === reason,
  );
  assert.equal(calls, 0);
  assert.equal(getEventListeners(controller.signal, 'abort').length, 0);
});

for (const lateResult of ['resolve', 'reject']) {
  test(
    `cancellation stops waiting, removes listeners and consumes a late ${lateResult}`,
    {
      timeout: 1000,
    },
    async () => {
      const prepared = prepare();
      const controller = new AbortController();
      const started = deferred();
      const pending = deferred();
      let secondCalls = 0;
      const secondKey = new Uint8Array(32).fill(9);
      const signing = prepared.sign(
        [
          {
            publicKey,
            signDigest(digest) {
              started.resolve(digest);
              return pending.promise;
            },
          },
          {
            publicKey: publicKeyFromPrivateKey(secondKey),
            async signDigest(digest) {
              secondCalls += 1;
              return signDigestCompact(digest, secondKey);
            },
          },
        ],
        {
          signal: controller.signal,
        },
      );
      const rejected = assert.rejects(signing, (error) => error === null);
      const digest = await started.promise;
      controller.abort(null);

      await rejected;
      assert.equal(getEventListeners(controller.signal, 'abort').length, 0);
      assert.equal(secondCalls, 0);

      if (lateResult === 'resolve') {
        pending.resolve(signDigestCompact(digest, privateKey));
      } else {
        pending.reject(new Error('late provider failure'));
      }
      await nextTurn();
      assert.equal(secondCalls, 0);
    },
  );
}

test('normal settlement removes listeners and provider errors keep their identity', async () => {
  const prepared = prepare();
  const controller = new AbortController();
  const options = {
    signal: controller.signal,
  };
  const signed = await prepared.sign(
    {
      publicKey,
      signDigest: async (digest) => signDigestCompact(digest, privateKey),
    },
    options,
  );
  assert.equal(signed.transaction.signatures.length, 1);
  assert.equal(getEventListeners(controller.signal, 'abort').length, 0);

  const reason = new Error('provider failure');
  await assert.rejects(
    prepared.sign(
      {
        publicKey,
        signDigest() {
          throw reason;
        },
      },
      options,
    ),
    (error) => error === reason,
  );
  assert.equal(getEventListeners(controller.signal, 'abort').length, 0);
});

test('a transaction expiring during signing is rejected before requesting another key', async (t) => {
  const now = Date.now();
  t.mock.method(Date, 'now', () => now);
  const prepared = prepare(now + 10000);
  let secondCalls = 0;

  await assert.rejects(
    prepared.sign([
      {
        publicKey,
        async signDigest(digest) {
          t.mock.method(Date, 'now', () => now + 20000);
          return signDigestCompact(digest, privateKey);
        },
      },
      {
        publicKey: publicKeyFromPrivateKey(new Uint8Array(32).fill(9)),
        async signDigest() {
          secondCalls += 1;
          return new Uint8Array(65);
        },
      },
    ]),
    signingError('expired'),
  );
  assert.equal(secondCalls, 0);
});

test('later signers cannot mutate earlier key or signature buffers', async () => {
  const prepared = prepare();
  const mutablePublicKey = Buffer.from(publicKey);
  const secondKey = new Uint8Array(32).fill(9);
  let firstSignature;
  const signed = await prepared.sign([
    {
      publicKey: mutablePublicKey,
      async signDigest(digest) {
        firstSignature = Buffer.from(signDigestCompact(digest, privateKey));
        mutablePublicKey.fill(0);
        return firstSignature;
      },
    },
    {
      publicKey: publicKeyFromPrivateKey(secondKey),
      async signDigest(digest) {
        firstSignature.fill(0);
        return signDigestCompact(digest, secondKey);
      },
    },
  ]);

  const digest = transactionDigest(prepared.chainId, prepared.bytes);
  const signatures = signed.transaction.signatures;
  for (const key of [publicKey, publicKeyFromPrivateKey(secondKey)]) {
    assert.ok(
      signatures.some((signature) =>
        verifyDigestCompact(digest, signature, key),
      ),
    );
  }
});

test('WIF public key mismatch has a stable signing error', async () => {
  await assert.rejects(
    prepare().signWithWifs([
      {
        wif: encodeWif(privateKey),
        expectedPublicKey: 'wrong-key',
      },
    ]),
    signingError('key-mismatch'),
  );
});
