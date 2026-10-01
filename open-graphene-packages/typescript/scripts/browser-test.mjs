import { build } from 'esbuild';
import { chromium } from '@playwright/test';
import { fileURLToPath } from 'node:url';

const result = await build({
  stdin: {
    contents: `
    import { TransactionCodec, encodeTransaction } from './graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
    import { signDigestCompact, recoverPublicKey, isCanonicalCompactSignature } from './graphene-fc/dist/signing.js';
    import { BitSharesWifSigner, BitSharesClient } from './graphene-chain-bitshares/graphene-chain-bitshares-api/dist/index.js';
    import { parseJson, stringifyJson } from './graphene-codec/dist/index.js';
    import { FcWriter, transactionDigest } from './graphene-fc/dist/index.js';
    import { bytesToHex, hexToBytes } from './graphene-primitives/dist/index.js';
    import fixture from './tests/fixtures/protocol-vectors.json';
    import memo from './tests/fixtures/memo-rust.json';
    import swaplockVectors from './tests/fixtures/swaplock-fc-parity-rich.json';
    import bitsharesVectors from './tests/fixtures/bitshares-fc-parity-rich.json';
    import * as swaplockBindings from './graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
    import * as bitsharesBindings from './graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';
    import { encryptMemo, decryptMemo } from './graphene-fc/dist/memo.js';
    globalThis.parity = async () => {
      const cipher = await encryptMemo(hexToBytes(memo.alicePrivateKeyHex), hexToBytes(memo.bobPublicKeyHex), BigInt(memo.nonce), new TextEncoder().encode(memo.message));
      if(bytesToHex(cipher)!==memo.ciphertextHex)throw new Error('Browser memo differs from Rust');
      const plain=await decryptMemo(hexToBytes(memo.bobPrivateKeyHex),hexToBytes(memo.alicePublicKeyHex),BigInt(memo.nonce),cipher);
      if(new TextDecoder().decode(plain)!==memo.message)throw new Error('Browser memo decryption');
      let checked=0;
      for(const [b,fixture] of [[swaplockBindings,swaplockVectors],[bitsharesBindings,bitsharesVectors]]) {
        for(const vector of fixture.vectors) {
          if(bytesToHex(b.encodeOperation(b.OperationCodec.decode(vector.operation)))!==vector.rust.hex)throw new Error('Browser FC differs: '+vector.name);
          checked++;
        }
      }
      return checked;
    };
    const tx = TransactionCodec.decode(fixture.transfer.transaction);
    const transferHex = bytesToHex(encodeTransaction(tx));
    const signature = signDigestCompact(hexToBytes(fixture.signing.digestHex), hexToBytes(fixture.signing.publicTestPrivateKeyHex));
    const legacySignature = signDigestCompact(hexToBytes(fixture.signing.digestHex), hexToBytes(fixture.signing.publicTestPrivateKeyHex), 'graphene-legacy');
    const bitsharesSigner = new BitSharesWifSigner('5HpjE2Hs7vjU4SN3YyPQCdhzCu92WoEeuE6PWNuiPyTu3ESGnzn');
    const bitsharesPublicKey = bytesToHex(bitsharesSigner.publicKey);
    bitsharesSigner.dispose();
    tx.operations[0][1].amount.amount = 9007199254740993n;
    const restored = TransactionCodec.decode(parseJson(stringifyJson(TransactionCodec.encode(tx))));
    const writer = new FcWriter(); writer.objectId('1.2.345');
    globalThis.result = {
      bitshares: typeof BitSharesClient.connect === 'function' && isCanonicalCompactSignature(legacySignature)
        && bytesToHex(recoverPublicKey(hexToBytes(fixture.signing.digestHex), legacySignature)) === bitsharesPublicKey,
      amount: restored.operations[0][1].amount.amount.toString(),
      id: bytesToHex(writer.finish()),
      digest: bytesToHex(transactionDigest(fixture.transfer.chainId, hexToBytes(fixture.transfer.hex))),
      expectedDigest: fixture.transfer.digestHex
      , transferHex, expectedTransferHex: fixture.transfer.hex,
      signature: bytesToHex(signature), expectedSignature: fixture.signing.signatureHex,
      recoveredKey: bytesToHex(recoverPublicKey(hexToBytes(fixture.signing.digestHex), signature)), expectedKey: fixture.signing.publicKeyHex
    };
  `,
    resolveDir: fileURLToPath(new URL('../', import.meta.url)),
    sourcefile: 'browser-smoke.ts',
  },
  bundle: true,
  platform: 'browser',
  target: 'es2022',
  format: 'iife',
  write: false,
});
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.route('https://sdk.test/', (route) =>
    route.fulfill({
      contentType: 'text/html',
      body: '<!doctype html><title>SDK test</title>',
    }),
  );
  await page.goto('https://sdk.test/');
  await page.addScriptTag({ content: result.outputFiles[0].text });
  const value = await page.evaluate(() => globalThis.result);
  if (
    !value.bitshares ||
    value.amount !== '9007199254740993' ||
    value.id !== '5901000000000201' ||
    value.digest !== value.expectedDigest ||
    value.transferHex !== value.expectedTransferHex ||
    value.signature !== value.expectedSignature ||
    value.recoveredKey !== value.expectedKey
  ) {
    throw new Error(`Browser vector mismatch: ${JSON.stringify(value)}`);
  }
  const parityCount = await page.evaluate(() => globalThis.parity());
  if (parityCount !== 159) throw new Error('Missing browser FC cases');
  console.log(
    'Chromium: generated JSON/FC, lossless integers, transaction digest, signing and recovery passed (no Node polyfills).',
  );
} finally {
  await browser.close();
}
