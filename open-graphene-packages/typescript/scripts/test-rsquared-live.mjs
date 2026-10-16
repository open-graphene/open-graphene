import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { RSquaredClient } from '../graphene-chain-rsquared/graphene-chain-rsquared-api/dist/index.js';
import {
  AccountId,
  ContentCardId,
  PermissionId,
} from '../graphene-chain-rsquared/graphene-chain-rsquared-bindings/dist/index.js';

const [output = '/tmp/rsquared-sdk-live.json', ...configured] =
  process.argv.slice(2);
const endpoints = configured.length
  ? configured
  : [
      'wss://node01.rsquared.digital:8090',
      'wss://node02.rsquared.digital:8090',
    ];

async function checkEndpoint(endpoint) {
  const report = {
    endpoint,
    checkedAt: new Date().toISOString(),
    checks: [],
  };
  let client;

  async function check(name, action) {
    try {
      const result = await action();
      report.checks.push({
        name,
        status: 'passed',
        result,
      });
    } catch (error) {
      report.checks.push({
        name,
        status: 'failed',
        reason: error.message,
      });
    }
  }

  try {
    client = await RSquaredClient.connect(endpoint, {
      timeoutMs: 10000,
    });
    report.chainId = client.chainId;

    await check('head and block decoding', async () => {
      const head = await client.database.getDynamicGlobalProperties({});
      const block = await client.database.getBlock({
        block_num: head.head_block_number,
      });
      assert.ok(block);
      assert.equal(block.timestamp, head.time);
      return {
        blockNumber: head.head_block_number,
        time: head.time,
      };
    });
    await check('core asset and account decoding', async () => {
      const asset = await client.database.asset('1.3.0');
      const account = await client.database.account('1.2.0');
      await client.database.accountBalances(account.id);
      await client.database.getFullAccounts({
        names_or_ids: [account.id],
        subscribe: false,
      });
      assert.equal(asset.symbol, 'RQRX');
      return {
        symbol: asset.symbol,
        account: account.name,
      };
    });
    await check('global properties and fee schedule decoding', async () => {
      const properties = await client.database.getGlobalProperties({});
      return {
        feeParameters: properties.parameters.current_fees.parameters.length,
      };
    });
    await check('personal data RPC', async () => {
      const personalData = await client.database.getPersonalData({
        subject_account: AccountId('1.2.0'),
        operator_account: AccountId('1.2.0'),
      });
      return {
        count: personalData.length,
      };
    });
    await check('content cards RPC', async () => {
      const contentCards = await client.database.getContentCards({
        subject_account: AccountId('1.2.0'),
        content_id: ContentCardId('1.20.0'),
        limit: 1,
      });
      return {
        count: contentCards.length,
      };
    });
    await check('permissions RPC', async () => {
      const permissions = await client.database.getPermissions({
        operator_account: AccountId('1.2.0'),
        permission_id: PermissionId('1.21.0'),
        limit: 1,
      });
      return {
        count: permissions.length,
      };
    });
    await check(
      'personal-data preparation and fee pricing without submission',
      async () => {
        const builder = client.operations.personal_data_create({
          subject_account: AccountId('1.2.0'),
          operator_account: AccountId('1.2.1'),
          url: 'https://example.com/data',
          hash: 'smoke-test',
          storage_data: '',
        });
        const prepared = await builder.prepare();
        const [operation] = prepared.transaction.operations;
        assert.equal(operation[0], 39);
        assert.ok(operation[1].fee.amount >= 0n);
        return {
          fee: operation[1].fee.amount.toString(),
          encodedBytes: prepared.bytes.length,
        };
      },
    );
    await check('head subscription receives a newer block', async () => {
      const subscription = await client.queries.watchDynamicGlobalProperties();
      try {
        const initial = await subscription.nextTimeout(15000);
        assert.equal(initial.done, false);
        const deadline = Date.now() + 15000;
        while (Date.now() < deadline) {
          const update = await subscription.nextTimeout(
            Math.max(1, deadline - Date.now()),
          );
          assert.equal(update.done, false);
          if (
            update.value.head_block_number > initial.value.head_block_number
          ) {
            return {
              initialBlock: initial.value.head_block_number,
              updatedBlock: update.value.head_block_number,
            };
          }
        }
        throw new Error('No newer block observed before timeout');
      } finally {
        subscription.close();
      }
    });
  } catch (error) {
    report.checks.push({
      name: 'connect and verify chain identity',
      status: 'failed',
      reason: error.message,
    });
  } finally {
    client?.close();
  }

  return report;
}

const runs = await Promise.all(endpoints.map(checkEndpoint));
await writeFile(output, JSON.stringify({ runs }, null, 2) + '\n');
console.log(JSON.stringify({ runs }, null, 2));
if (runs.some((run) => run.checks.some((check) => check.status === 'failed'))) {
  process.exitCode = 1;
}
