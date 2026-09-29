import { build } from 'esbuild';
import { chromium } from '@playwright/test';
import { fileURLToPath } from 'node:url';

const result = await build({
  stdin: { contents: `
    import { TransactionCodec, encodeTransaction } from './graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
    import { signDigestCompact, recoverPublicKey } from './graphene-fc/dist/signing.js';
    import { parseJson, stringifyJson } from './graphene-codec/dist/index.js';
    import { FcWriter, transactionDigest } from './graphene-fc/dist/index.js';
    import { bytesToHex, hexToBytes } from './graphene-primitives/dist/index.js';
    import fixture from './tests/fixtures/protocol-vectors.json';
    const tx = TransactionCodec.decode(fixture.transfer.transaction);
    const transferHex = bytesToHex(encodeTransaction(tx));
    const signature = signDigestCompact(hexToBytes(fixture.signing.digestHex), hexToBytes(fixture.signing.publicTestPrivateKeyHex));
    tx.operations[0][1].amount.amount = 9007199254740993n;
    const restored = TransactionCodec.decode(parseJson(stringifyJson(TransactionCodec.encode(tx))));
    const writer = new FcWriter(); writer.objectId('1.2.345');
    globalThis.result = {
      amount: restored.operations[0][1].amount.amount.toString(),
      id: bytesToHex(writer.finish()),
      digest: bytesToHex(transactionDigest(fixture.transfer.chainId, hexToBytes(fixture.transfer.hex))),
      expectedDigest: fixture.transfer.digestHex
      , transferHex, expectedTransferHex: fixture.transfer.hex,
      signature: bytesToHex(signature), expectedSignature: fixture.signing.signatureHex,
      recoveredKey: bytesToHex(recoverPublicKey(hexToBytes(fixture.signing.digestHex), signature)), expectedKey: fixture.signing.publicKeyHex
    };
  `, resolveDir: fileURLToPath(new URL('../', import.meta.url)), sourcefile: 'browser-smoke.ts' },
  bundle: true, platform: 'browser', target: 'es2022', format: 'iife', write: false,
});
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.addScriptTag({ content: result.outputFiles[0].text });
  const value = await page.evaluate(() => globalThis.result);
  if (value.amount !== '9007199254740993' || value.id !== '5901000000000201' || value.digest !== value.expectedDigest || value.transferHex !== value.expectedTransferHex || value.signature !== value.expectedSignature || value.recoveredKey !== value.expectedKey) {
    throw new Error(`Browser vector mismatch: ${JSON.stringify(value)}`);
  }
  console.log('Chromium: generated JSON/FC, lossless integers, transaction digest, signing and recovery passed (no Node polyfills).');
} finally { await browser.close(); }
