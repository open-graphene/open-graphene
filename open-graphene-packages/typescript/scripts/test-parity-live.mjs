// Explicit opt-in: small Swaplock testnet transactions. Never uses BitShares mainnet.
import { readFile, writeFile } from 'node:fs/promises';
import {
  SwaplockClient,
  RoomAccessPrecondition,
  memberAddOperation,
  memberRemoveOperation,
} from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { RpcRemoteError } from '../graphene-transport/dist/index.js';
import {
  WifSigner,
  decodeWif,
  encodePublicKey,
} from '../graphene-fc/dist/signing.js';
import {
  encryptMemo,
  decryptMemo,
  uniqueNonce,
} from '../graphene-fc/dist/memo.js';
import { bytesToHex } from '../graphene-primitives/dist/index.js';
import { stringifyJson, unknownValue } from '../graphene-codec/dist/index.js';
const [genesisPath, path = '/tmp/swaplock-typescript-parity-live.json'] =
  process.argv.slice(2);
if (!genesisPath)
  throw Error(
    'Provide local genesis file path to authorize testnet transactions',
  );
const genesis = JSON.parse(await readFile(genesisPath, 'utf8'));
const records = ['swaplock', 'registrar'].map((name) =>
  genesis.initial_accounts.find((a) => a.name === name),
);
const signers = records.map(
  (r) => new WifSigner(r.active_key_full.wif_priv_key),
);
const c = await SwaplockClient.connect(
  'wss://node01.swaplock.chainpool.online:8090',
);
let report;
try {
  report = JSON.parse(await readFile(path, 'utf8'));
} catch (e) {
  if (e.code !== 'ENOENT') throw e;
  report = {
    startedAt: new Date().toISOString(),
    chainId: c.chainId,
    transactions: [],
    checks: [],
  };
}
const save = () =>
  writeFile(path, stringifyJson(unknownValue.encode(report)) + '\n');
const check = (name, ok) => {
  if (!ok) throw Error(name);
  report.checks.push({ name, status: 'passed' });
};
const streams = [];
const bare = b.bindOperationBuilders((op) => op);
async function send(label, operations) {
  const previous = report.transactions.find((t) => t.label === label);
  if (previous) {
    if (previous.status === 'included') return previous;
    throw Error(
      'Unresolved prior submission; inspect before any retry: ' + previous.id,
    );
  }
  const prepared = await c.prepareOperations(operations, { maxFee: 1000000n });
  const signed = await prepared.sign([signers[0], signers[0]]);
  check(
    label + ': deduplicated signing key',
    signed.transaction.signatures.length === 1,
  );
  const native = await c.rpc.request('database', 'get_transaction_hex', [
    signed.toJSON(),
  ]);
  check(
    label + ': native signed FC',
    native === bytesToHex(b.encodeSignedTransaction(signed.transaction)),
  );
  check(
    label + ': node authority',
    (await c.rpc.request('database', 'verify_authority', [signed.toJSON()])) ===
      true,
  );
  const entry = {
    label,
    id: signed.id,
    status: 'prepared',
    feeUnits: signed.transaction.operations
      .reduce((sum, op) => sum + op[1].fee.amount, 0n)
      .toString(),
  };
  report.transactions.push(entry);
  await save();
  let pending;
  try {
    pending = await c.networkBroadcast.sendTransactionWithCallback(signed);
  } catch (e) {
    if (e instanceof RpcRemoteError) {
      entry.status = 'rejected';
      entry.reason = e.detail?.message ?? e.message;
      await save();
    }
    throw e;
  }
  entry.status = 'submitted';
  await save();
  const confirmation = await pending.wait(30000);
  const inclusion = await c.waitForInclusion(signed, 30000);
  check(
    label + ': callback matches inclusion',
    confirmation.blockNumber === inclusion.blockNumber,
  );
  const history = await c.history.getAccountHistory({
    account_name_or_id: 'swaplock',
    stop: '1.11.0',
    limit: 100,
    start: '1.11.0',
  });
  const applied = history
    .filter(
      (h) =>
        h.block_num === inclusion.blockNumber &&
        h.trx_in_block === inclusion.transactionIndex &&
        !h.is_virtual,
    )
    .sort((a, b) => a.op_in_trx - b.op_in_trx);
  check(
    label + ': all operations recorded',
    applied.length === operations.length,
  );
  Object.assign(entry, {
    status: 'included',
    blockNumber: inclusion.blockNumber,
    results: applied.map((h) => b.OperationResultCodec.encode(h.result)),
  });
  await save();
  return entry;
}
try {
  const accounts = await c.database.getAccounts({
    account_names_or_ids: ['swaplock', 'registrar'],
    subscribe: false,
  });
  const sender = accounts[0],
    recipient = accounts[1];
  check(
    'genesis active authority',
    sender.active.key_auths.some(
      ([key, weight]) =>
        key === encodePublicKey(signers[0].publicKey, 'BTS') &&
        weight >= sender.active.weight_threshold,
    ),
  );
  const heads = await c.database.watchDynamicGlobalProperties();
  streams.push(heads);
  const balances = await c.database.watchBalances('swaplock');
  streams.push(balances);
  const history = await c.history.watchAccountHistory('swaplock', 5);
  streams.push(history);
  const market = await c.database.subscribeMarket(
    b.AssetId('1.3.100'),
    b.AssetId('1.3.0'),
  );
  streams.push(market);
  check('initial balance snapshot', !(await balances.nextTimeout(5000)).done);
  check('initial history snapshot', !(await history.nextTimeout(5000)).done);
  const firstHead = (await heads.nextTimeout(5000)).value.head_block_number;
  const store = await c.chainStore(['2.1.0']);
  streams.push(store);
  const nonce = uniqueNonce(),
    message = new TextEncoder().encode(
      'Open Graphene native TypeScript parity test',
    );
  const senderKey = decodeWif(records[0].active_key_full.wif_priv_key),
    recipientKey = decodeWif(records[1].active_key_full.wif_priv_key);
  let encrypted;
  try {
    encrypted = await encryptMemo(
      senderKey,
      signers[1].publicKey,
      nonce,
      message,
    );
    check(
      'memo recipient decrypts',
      bytesToHex(
        await decryptMemo(recipientKey, signers[0].publicKey, nonce, encrypted),
      ) === bytesToHex(message),
    );
  } finally {
    senderKey.fill(0);
    recipientKey.fill(0);
  }
  const setup = await send('compound memo transfer, strict room and order', [
    bare.transfer({
      from: sender.id,
      to: recipient.id,
      amount: { amount: 1n, asset_id: b.AssetId('1.3.0') },
      memo: {
        from: encodePublicKey(signers[0].publicKey, 'BTS'),
        to: encodePublicKey(signers[1].publicKey, 'BTS'),
        nonce,
        message: encrypted,
      },
    }),
    bare.data_room_create({
      owner: sender.id,
      name: 'ts-parity-' + Date.now().toString(36),
      description: 'Public dummy SDK fixture',
      subject: [0, {}],
      room_key: 'public-fixture-envelope',
      extensions: { write_policy: 1 },
    }),
    bare.limit_order_create({
      seller: sender.id,
      amount_to_sell: { amount: 1n, asset_id: b.AssetId('1.3.100') },
      min_to_receive: { amount: 2n, asset_id: b.AssetId('1.3.0') },
      expiration: new Date(Date.now() + 3600000).toISOString().slice(0, 19),
      fill_or_kill: false,
    }),
  ]);
  report.roomId = setup.results[1][1];
  report.orderId = setup.results[2][1];
  await save();
  check(
    'market notice after native TS order',
    !(await market.nextTimeout(15000)).done,
  );
  check(
    'balance notification after transaction',
    !(await balances.nextTimeout(15000)).done,
  );
  check(
    'history notification after transaction',
    !(await history.nextTimeout(15000)).done,
  );
  check(
    'head notification',
    (await heads.nextTimeout(15000)).value.head_block_number > firstHead,
  );
  await store.changes.nextTimeout(15000);
  check('ChainStore receives live objects', store.size === 1);
  const state = await c.database.getDataRoomAccessState({
    room_id: report.roomId,
  });
  const guarded = RoomAccessPrecondition.fromSnapshot(state).guard(
    memberAddOperation(
      sender.id,
      report.roomId,
      recipient.id,
      'public-recipient-fixture-envelope',
    ),
  );
  await send('guarded room member add', [guarded]);
  const member = await c.database.getDataRoomMember({
    room_id: report.roomId,
    member_name_key_or_id: recipient.id,
  });
  check('member added under access guard', member !== null);
  const nextState = await c.database.getDataRoomAccessState({
    room_id: report.roomId,
  });
  await send('remove fixture member, cancel order and delete room', [
    RoomAccessPrecondition.fromSnapshot(nextState).guard(
      memberRemoveOperation(sender.id, report.roomId, recipient.id),
    ),
    bare.limit_order_cancel({
      fee_paying_account: sender.id,
      order: report.orderId,
    }),
    bare.data_room_delete({ caller: sender.id, room: report.roomId }),
  ]);
  const objects = await c.database.getObjects({
    ids: [report.roomId, report.orderId],
    subscribe: false,
  });
  check(
    'fixture room and order removed',
    objects.every((v) => v === null),
  );
  for (const stream of streams) stream.close();
  await c.reconnect();
  check('reconnect same chain', c.chainId === b.CHAIN.chainId);
  report.finishedAt = new Date().toISOString();
  await save();
  console.log(
    JSON.stringify(
      { transactions: report.transactions, checks: report.checks },
      null,
      2,
    ),
  );
} finally {
  for (const stream of streams) stream.close();
  for (const signer of signers) signer.dispose();
  c.close();
  await save();
}
