import { readFile, writeFile } from 'node:fs/promises';
import { fixtureWriter } from '../tests/live/fixture-writer.mjs';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
const [genesisPath, output = '/tmp/swaplock-sdk-fixtures.json'] =
  process.argv.slice(2);
if (!genesisPath) throw new Error('Provide local genesis file path');
const writer = await fixtureWriter(genesisPath, output + '.journal.json');
const fee = { amount: 0n, asset_id: b.AssetId('1.3.0') };
let fixture;
try {
  fixture = JSON.parse(await readFile(output, 'utf8'));
} catch (e) {
  if (e.code !== 'ENOENT') throw e;
  fixture = {
    symbol: 'OGTS' + Date.now().toString(36).toUpperCase(),
    createdAt: new Date().toISOString(),
    serialization: 'native node FC, fixture setup only',
  };
}
const save = () => writeFile(output, JSON.stringify(fixture, null, 2) + '\n');
try {
  await save();
  const options = {
    max_supply: 1000000000n,
    market_fee_percent: 0,
    max_market_fee: 0n,
    issuer_permissions: 0,
    flags: 0,
    core_exchange_rate: {
      base: { amount: 1n, asset_id: b.AssetId('1.3.0') },
      quote: { amount: 1n, asset_id: b.AssetId('1.3.1') },
    },
    whitelist_authorities: [],
    blacklist_authorities: [],
    whitelist_markets: [],
    blacklist_markets: [],
    description: 'Open Graphene SDK integration test asset; no economic value',
    extensions: {},
  };
  if (!fixture.assetId) {
    const created = await writer.send(
      b.operation.asset_create({
        fee,
        issuer: writer.account.id,
        symbol: fixture.symbol,
        precision: 5,
        common_options: options,
        is_prediction_market: false,
        extensions: [],
      }),
      'create isolated market test asset',
    );
    fixture.assetId = created[1];
    await save();
  }
  await writer.send(
    b.operation.asset_issue({
      fee,
      issuer: writer.account.id,
      asset_to_issue: { amount: 100000n, asset_id: fixture.assetId },
      issue_to_account: writer.account.id,
      extensions: [],
    }),
    'issue one test token',
  );
  const expiration = new Date(Date.now() + 3600000).toISOString().slice(0, 19);
  const order = (sellAsset, sell, receiveAsset, receive) =>
    b.operation.limit_order_create({
      fee,
      seller: writer.account.id,
      amount_to_sell: { amount: sell, asset_id: sellAsset },
      min_to_receive: { amount: receive, asset_id: receiveAsset },
      expiration,
      fill_or_kill: false,
      extensions: {},
    });
  await writer.send(
    order(fixture.assetId, 100n, '1.3.0', 100n),
    'maker order for test fill',
  );
  await writer.send(
    order('1.3.0', 100n, fixture.assetId, 100n),
    'taker order for test fill',
  );
  const resting = await writer.send(
    order(fixture.assetId, 200n, '1.3.0', 300n),
    'resting order for depth/grouping reads',
  );
  fixture.orderId = resting[1];
  await save();
  const room = await writer.send(
    b.operation.data_room_create({
      fee,
      owner: writer.account.id,
      name: 'Open Graphene SDK test ' + fixture.symbol,
      description: 'Disposable SDK test fixture',
      subject: [1, fixture.assetId],
      extensions: {},
    }),
    'room with asset subject',
  );
  fixture.subjectRoomId = room[1];
  await save();
  const privateRoom = await writer.send(
    b.operation.data_room_create({
      fee,
      owner: writer.account.id,
      name: 'SDK grant fixture ' + fixture.symbol,
      description: 'Test data only; public dummy envelope',
      subject: [1, fixture.assetId],
      room_key: 'public-sdk-fixture-envelope-not-a-secret',
      extensions: {},
    }),
    'isolated encrypted room for grant reads',
  );
  fixture.grantRoomId = privateRoom[1];
  await save();
  const card = await writer.send(
    b.operation.content_card_create({
      fee,
      payer: writer.account.id,
      author: [0, writer.account.id],
      room: fixture.grantRoomId,
      hash: '11'.repeat(32),
      url: 'https://example.invalid/sdk-fixture',
      type: 'sdk-test',
      description: 'Public dummy SDK fixture; no confidential content',
      content_key: 'public-sdk-fixture-content-key',
      storage_data: '{}',
      extensions: [],
    }),
    'isolated content card with storage metadata for grant reads',
  );
  fixture.grantCardId = card[1];
  await save();
  const grant = await writer.send(
    b.operation.content_card_grant_create({
      fee,
      granter: writer.account.id,
      content_id: fixture.grantCardId,
      grantee: [0, b.AccountId('1.2.101')],
      key: 'public-sdk-fixture-grant-key',
      extensions: [],
    }),
    'grant to controlled registrar account',
  );
  fixture.grantId = grant[1];
  await save();
  console.log(JSON.stringify(fixture, null, 2));
} finally {
  await writer.close();
}
