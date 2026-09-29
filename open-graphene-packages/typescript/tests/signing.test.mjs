import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { bytesToHex, hexToBytes } from '../graphene-primitives/dist/index.js';
import { transactionDigest } from '../graphene-fc/dist/index.js';
import { signDigestCompact, recoverPublicKey, verifyDigestCompact, WifSigner, encodePublicKey } from '../graphene-fc/dist/signing.js';
import { PreparedTransfer } from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
const fixture = JSON.parse(readFileSync(new URL('fixtures/protocol-vectors.json', import.meta.url)));

test('public WIF fixture decodes, signs and disposes without exposing private bytes', async () => {
  const signer = new WifSigner('5HpjE2Hs7vjU4SN3YyPQCdhzCu92WoEeuE6PWNuiPyTu3ESGnzn');
  assert.equal(bytesToHex(signer.publicKey), fixture.signing.publicKeyHex);
  assert.equal(bytesToHex(await signer.signDigest(hexToBytes(fixture.signing.digestHex))), fixture.signing.signatureHex);
  assert.equal(JSON.stringify(signer).includes('5Hpj'), false);
  signer.dispose();
  await assert.rejects(signer.signDigest(new Uint8Array(32)), /disposed/);
});

test('generated FC transaction bytes and digest equal shared Rust vectors', () => {
  const tx = b.TransactionCodec.decode(fixture.transfer.transaction);
  assert.equal(bytesToHex(b.encodeTransaction(tx)), fixture.transfer.hex);
  assert.equal(bytesToHex(transactionDigest(fixture.transfer.chainId, b.encodeTransaction(tx))), fixture.transfer.digestHex);
  assert.equal(bytesToHex(b.encodeSignedTransaction({ ...tx, signatures: [] })), fixture.transfer.hex + '00');
  assert.throws(() => b.encodeTransaction({ ...tx, extensions: [[0, {}]] }), /Nonempty future extensions/);
});
test('Swaplock low-S signature matches Rust, recovers key, rejects tampering', () => {
  const digest = hexToBytes(fixture.signing.digestHex);
  const key = hexToBytes(fixture.signing.publicTestPrivateKeyHex);
  const signature = signDigestCompact(digest, key);
  assert.equal(bytesToHex(signature), fixture.signing.signatureHex);
  assert.equal(bytesToHex(recoverPublicKey(digest, signature)), fixture.signing.publicKeyHex);
  const publicKey = hexToBytes(fixture.signing.publicKeyHex);
  assert.equal(verifyDigestCompact(digest, signature, publicKey), true);
  const tampered = digest.slice(); tampered[0] ^= 1;
  assert.equal(verifyDigestCompact(tampered, signature, publicKey), false);
  const badHeader = signature.slice(); badHeader[0] = 30;
  assert.equal(verifyDigestCompact(digest, badHeader, publicKey), false);
  assert.throws(() => signDigestCompact(new Uint8Array(31), key));
});
test('prepared transaction is isolated from mutations and validates external signer', async () => {
  const digestKey = hexToBytes(fixture.signing.publicTestPrivateKeyHex);
  const publicKey = hexToBytes(fixture.signing.publicKeyHex);
  const authority = b.AuthorityCodec.decode({ weight_threshold: 1, account_auths: [], key_auths: [[encodePublicKey(publicKey, 'BTS'), 1]], address_auths: [] });
  const tx = b.TransactionCodec.decode({ ...fixture.transfer.transaction, expiration: new Date(Date.now() + 60000).toISOString().slice(0, 19) });
  const prepared = new PreparedTransfer(tx, authority, 1);
  const original = bytesToHex(prepared.bytes);
  tx.operations[0][1].amount.amount = 99n;
  prepared.transaction.operations[0][1].amount.amount = 100n;
  authority.key_auths.length = 0;
  assert.equal(bytesToHex(prepared.bytes), original);
  await assert.rejects(prepared.sign({ publicKey, signDigest: async () => new Uint8Array(65) }), /invalid signature/);
  const signed = await prepared.sign({ publicKey, signDigest: async digest => signDigestCompact(digest, digestKey) });
  assert.equal(signed.id, prepared.id);
  const before = JSON.stringify(signed);
  signed.transaction.signatures[0].fill(0);
  assert.equal(JSON.stringify(signed), before);
  assert.throws(() => new WifSigner('invalid'), /Invalid Graphene WIF/);
});
