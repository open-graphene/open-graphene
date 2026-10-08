import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import {
  SignedTransfer,
  restoreSignedTransaction,
  TransactionRestorationError,
  transactionId,
} from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import { transactionDigest } from '../graphene-fc/dist/index.js';
import {
  signDigestCompact,
  publicKeyFromPrivateKey,
} from '../graphene-fc/dist/signing.js';
import { stringifyJson } from '../graphene-codec/dist/index.js';

const vectors = JSON.parse(
  readFileSync(new URL('fixtures/protocol-vectors.json', import.meta.url)),
);
const privateKey = Buffer.from(vectors.signing.publicTestPrivateKeyHex, 'hex');
const publicKey = Buffer.from(vectors.signing.publicKeyHex, 'hex');
const chainId = 'ab'.repeat(32);

function fixture() {
  const transaction = b.TransactionCodec.decode(vectors.transfer.transaction);
  const digest = transactionDigest(chainId, b.encodeTransaction(transaction));
  const signature = signDigestCompact(digest, privateKey);
  const signed = new SignedTransfer(
    {
      ...transaction,
      signatures: [signature],
    },
    100,
  );
  const options = {
    serializedTransaction: stringifyJson(signed.toJSON()),
    chainId,
    expectedTransactionId: signed.id,
    expectedPublicKey: publicKey,
    startBlock: signed.startBlock,
  };
  return {
    signed,
    options,
  };
}

function restorationError(code) {
  return (error) =>
    error instanceof TransactionRestorationError && error.code === code;
}

test('restoration verifies an expired transaction and preserves its signed bytes for recovery', () => {
  const state = fixture();
  assert.ok(Date.parse(state.signed.transaction.expiration + 'Z') < Date.now());
  const restored = restoreSignedTransaction(state.options);
  assert.equal(restored.id, state.signed.id);
  assert.equal(restored.startBlock, 100);
  assert.deepEqual(
    b.encodeSignedTransaction(restored.transaction),
    b.encodeSignedTransaction(state.signed.transaction),
  );

  const exposed = restored.transaction;
  exposed.signatures[0].fill(0);
  exposed.operations[0][1].amount.amount = 1n;
  assert.deepEqual(restored.toJSON(), state.signed.toJSON());
});

for (const serialized of ['{private-marker', 'null', '[]', '{}']) {
  test(`malformed record ${serialized} has a typed error without disclosing its content`, () => {
    const state = fixture();
    state.options.serializedTransaction = serialized;
    assert.throws(
      () => restoreSignedTransaction(state.options),
      (error) => {
        assert.ok(restorationError('invalid-transaction')(error));
        assert.equal(error.message, 'Invalid serialized signed transaction');
        assert.equal(error.cause, undefined);
        return true;
      },
    );
  });
}

test('a changed expected ID is rejected', () => {
  const state = fixture();
  state.options.expectedTransactionId = 'ff'.repeat(20);
  assert.throws(
    () => restoreSignedTransaction(state.options),
    restorationError('transaction-mismatch'),
  );
});

for (const updateId of [false, true]) {
  test(`a changed transaction body is rejected even when updateId=${updateId}`, () => {
    const state = fixture();
    const wire = state.signed.toJSON();
    wire.ref_block_num = 999;
    state.options.serializedTransaction = stringifyJson(wire);
    if (updateId) {
      state.options.expectedTransactionId = transactionId(
        b.TransactionCodec.decode(wire),
      );
    }
    assert.throws(
      () => restoreSignedTransaction(state.options),
      restorationError(updateId ? 'invalid-signature' : 'transaction-mismatch'),
    );
  });
}

for (const altered of ['key', 'chain', 'signature', 'missing', 'extra']) {
  test(`altered ${altered} cannot restore a transaction with its original ID`, () => {
    const state = fixture();
    const wire = state.signed.toJSON();
    if (altered === 'key') {
      state.options.expectedPublicKey = publicKeyFromPrivateKey(
        new Uint8Array(32).fill(9),
      );
    } else if (altered === 'chain') {
      state.options.chainId = 'cd'.repeat(32);
    } else if (altered === 'signature') {
      wire.signatures = ['00'.repeat(65)];
    } else if (altered === 'missing') {
      wire.signatures = [];
    } else {
      wire.signatures.push(wire.signatures[0]);
    }
    state.options.serializedTransaction = stringifyJson(wire);
    assert.throws(
      () => restoreSignedTransaction(state.options),
      restorationError('invalid-signature'),
    );
  });
}

for (const [field, value] of [
  ['chainId', 'invalid'],
  ['expectedTransactionId', 'invalid'],
  ['expectedPublicKey', new Uint8Array(32)],
  ['startBlock', -1],
  ['startBlock', 0.5],
  ['startBlock', 4294967296],
]) {
  test(`invalid ${field} has a stable input error`, () => {
    const state = fixture();
    state.options[field] = value;
    assert.throws(
      () => restoreSignedTransaction(state.options),
      restorationError('invalid-input'),
    );
  });
}
