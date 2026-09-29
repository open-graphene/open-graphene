// Explicit opt-in testnet example. Genesis is read locally; private fields are
// never included in requests, reports, exceptions or environment variables.
import { readFile, writeFile } from 'node:fs/promises';
import { parseArgs } from 'node:util';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { SwaplockClient, BroadcastOutcomeUnknown } from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import { WifSigner, encodePublicKey } from '../graphene-fc/dist/signing.js';
import { bytesToHex } from '../graphene-primitives/dist/index.js';
import { stringifyJson, unknownValue } from '../graphene-codec/dist/index.js';

const { values } = parseArgs({ options: {
  genesis: { type: 'string' }, broadcast: { type: 'boolean', default: false },
  report: { type: 'string' }, endpoint: { type: 'string', default: 'wss://node01.swaplock.chainpool.online:8090' },
  verifyEndpoint: { type: 'string', default: 'wss://node02.swaplock.chainpool.online:8090' },
} });
if (!values.genesis) throw new Error('Provide --genesis /local/path/genesis.private.json');
const genesis = JSON.parse(await readFile(values.genesis, 'utf8'));
const client = await SwaplockClient.connect(values.endpoint);
let signer;
let verifier;
const report = { startedAt: new Date().toISOString(), endpoint: values.endpoint, chainId: b.CHAIN.chainId, mode: values.broadcast ? 'broadcast' : 'dry-run' };
const persist = async () => { if (values.report) await writeFile(values.report, stringifyJson(unknownValue.encode(report)) + '\n'); };
try {
  const [asset] = await client.rpc.invoke(b.DatabaseGetAssets, { asset_symbols_or_ids: ['1.3.0'], subscribe: false });
  if (!asset) throw new Error('Core asset not found');
  const amount = 10n ** BigInt(Math.max(0, asset.precision - 2)); // 0.01 display units when precision >= 2.
  const maxFee = 3n * 10n ** BigInt(asset.precision);
  const names = genesis.initial_accounts.map(account => account.name);
  const accounts = await client.rpc.invoke(b.DatabaseGetAccounts, { account_names_or_ids: names, subscribe: false });
  let from;
  for (let i = 0; i < accounts.length; i++) {
    const account = accounts[i]; const local = genesis.initial_accounts[i];
    if (!account || !local.active_key_full?.wif_priv_key) continue;
    const balances = await client.rpc.invoke(b.DatabaseGetAccountBalances, { account_name_or_id: account.id, assets: [asset.id] });
    if ((balances[0]?.amount ?? 0n) < amount + maxFee) continue;
    const candidate = new WifSigner(local.active_key_full.wif_priv_key);
    const pub = encodePublicKey(candidate.publicKey, b.CHAIN.publicKeyPrefix);
    if ((account.active.key_auths.find(([key]) => key === pub)?.[1] ?? 0) < account.active.weight_threshold) { candidate.dispose(); continue; }
    signer = candidate; from = account; break;
  }
  if (!from || !signer) throw new Error('No funded genesis account with matching single-key active authority');
  const to = accounts.find(account => account && account.id !== from.id);
  if (!to) throw new Error('No distinct genesis recipient account');
  const balance = async account => (await client.rpc.invoke(b.DatabaseGetAccountBalances, { account_name_or_id: account.id, assets: [asset.id] }))[0]?.amount ?? 0n;
  const before = { from: await balance(from), to: await balance(to) };
  const prepared = await client.prepareTransfer({ from: from.id, to: to.id, amount, maxFee });
  const signed = await prepared.sign(signer);
  signer.dispose();
  const localHex = bytesToHex(b.encodeSignedTransaction(signed.transaction));
  // Independent native C++ oracle: compare its serialization and authority check.
  const nodeHex = await client.rpc.request('database', 'get_transaction_hex', [signed.toJSON()]);
  if (nodeHex !== localHex) throw new Error('Native C++ and TypeScript signed transaction bytes differ');
  if (await client.rpc.request('database', 'verify_authority', [signed.toJSON()]) !== true) throw new Error('Node rejected signature authority');
  Object.assign(report, {
    transactionId: signed.id, from: { id: from.id, name: from.name }, to: { id: to.id, name: to.name },
    asset: { id: asset.id, symbol: asset.symbol, precision: asset.precision },
    amountUnits: amount.toString(), feeUnits: prepared.transaction.operations[0][1].fee.amount.toString(),
    nativeFcBytesMatch: true, nodeAuthorityVerified: true,
    signedTransaction: signed.toJSON(), unsignedFcHex: bytesToHex(prepared.bytes), signedFcHex: localHex,
  });
  await persist(); // Save public transaction identity before the irreversible send.
  if (values.broadcast) {
    try { report.submission = await client.broadcast(signed); }
    catch (error) {
      if (!(error instanceof BroadcastOutcomeUnknown)) throw error;
      report.submission = { transactionId: signed.id, status: 'unknown; checking inclusion without rebroadcast' };
    }
    await persist();
    report.inclusion = await client.waitForInclusion(signed);
    await persist();
    verifier = await SwaplockClient.connect(values.verifyEndpoint);
    report.secondNodeInclusion = { endpoint: values.verifyEndpoint, ...await verifier.waitForInclusion(signed) };
    const after = { from: await balance(from), to: await balance(to) };
    report.balanceChanges = { senderUnits: (after.from - before.from).toString(), recipientUnits: (after.to - before.to).toString() };
    const deadline = Date.now() + 60000;
    while (Date.now() < deadline) {
      const head = await verifier.rpc.invoke(b.DatabaseGetDynamicGlobalProperties, {});
      if (head.last_irreversible_block_num >= report.inclusion.blockNumber) { report.lastIrreversibleBlock = head.last_irreversible_block_num; break; }
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    report.explorer = `https://portal.swaplock.chainpool.online/block/${report.inclusion.blockNumber}`;
  }
  report.finishedAt = new Date().toISOString();
  await persist();
  const { signedTransaction, unsignedFcHex, signedFcHex, ...summary } = report;
  console.log(JSON.stringify(summary, null, 2));
} finally { signer?.dispose(); verifier?.close(); client.close(); }
