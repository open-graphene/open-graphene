import { BitSharesClient } from '../../graphene-chain-bitshares/graphene-chain-bitshares-api/dist/index.js';
import * as b from '../../graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';
import {
  bytesToHex,
  parseTimePointSec,
} from '../../graphene-primitives/dist/index.js';
export async function runBitSharesSmoke(endpoint) {
  const client = await BitSharesClient.connect(endpoint);
  const report = {
    endpoint,
    chainId: b.CHAIN.chainId,
    startedAt: new Date().toISOString(),
    checks: [],
    broadcast: false,
  };
  const expect = (value, message) => {
    if (!value) throw new Error(message);
  };
  const call = async (descriptor, params = {}, check = () => {}) => {
    const result = await client.rpc.invoke(descriptor, params);
    check(result);
    report.checks.push({
      method: descriptor.api + '.' + descriptor.method,
      status: 'passed',
    });
    return result;
  };
  try {
    await call(b.DatabaseGetChainId, {}, (id) =>
      expect(id === b.CHAIN.chainId, 'Chain mismatch'),
    );
    const head = await call(b.DatabaseGetDynamicGlobalProperties);
    report.headBlock = head.head_block_number;
    const accounts = await call(
      b.DatabaseGetAccounts,
      {
        account_names_or_ids: [
          'committee-account',
          'witness-account',
          '1.2.281474976710655',
        ],
        subscribe: false,
      },
      (v) =>
        expect(
          v[0]?.name === 'committee-account' &&
            v[1]?.name === 'witness-account' &&
            v[2] === null,
          'Account/null slots',
        ),
    );
    await call(
      b.DatabaseGetObjects,
      { ids: ['1.3.0', '1.2.281474976710655'], subscribe: false },
      (v) => expect(v[0]?.kind === 'asset' && v[1] === null, 'Object routing'),
    );
    await call(b.DatabaseGetAccountBalances, {
      account_name_or_id: accounts[0].id,
      assets: ['1.3.0'],
    });
    await call(
      b.DatabaseGetBlockHeader,
      { block_num: head.head_block_number },
      (v) => expect(v !== null, 'Missing header'),
    );
    await call(b.DatabaseGetBlock, { block_num: head.head_block_number }, (v) =>
      expect(v !== null, 'Missing block'),
    );
    await call(b.DatabaseGetLimitOrders, {
      a: '1.3.0',
      b: '1.3.121',
      limit: 5,
    });
    await call(b.HistoryGetAccountHistory, {
      account_name_or_id: accounts[0].id,
      stop: '1.11.0',
      limit: 5,
      start: '1.11.0',
    });
    let op = b.operation.transfer({
      from: accounts[0].id,
      to: accounts[1].id,
      amount: { amount: 1n, asset_id: b.AssetId('1.3.0') },
      fee: { amount: 0n, asset_id: b.AssetId('1.3.0') },
      extensions: [],
    });
    const fees = await call(
      b.DatabaseGetRequiredFees,
      { ops: [op], asset_symbol_or_id: '1.3.0' },
      (v) => expect(v[0].amount >= 0n, 'Fee response'),
    );
    op = b.operation.transfer({ ...op[1], fee: fees[0] });
    const tx = {
      ref_block_num: head.head_block_number & 0xffff,
      ref_block_prefix: new DataView(
        head.head_block_id.buffer,
        head.head_block_id.byteOffset,
        20,
      ).getUint32(4, true),
      expiration: new Date((parseTimePointSec(head.time) + 120) * 1000)
        .toISOString()
        .slice(0, 19),
      operations: [op],
      extensions: [],
    };
    const nativeHex = await client.rpc.request(
      'database',
      'get_transaction_hex_without_sig',
      [b.TransactionCodec.encode(tx)],
    );
    expect(
      nativeHex === bytesToHex(b.encodeTransaction(tx)),
      'Native FC mismatch',
    );
    report.checks.push({
      method: 'transfer FC vs native C++',
      status: 'passed',
    });
    report.unsignedTransaction = b.TransactionCodec.encode(tx);
    report.unsignedFcHex = nativeHex;
    for (const [descriptor, params] of [
      [b.DatabaseGetChainProperties, {}],
      [b.DatabaseGetGlobalProperties, {}],
      [b.DatabaseGetConfig, {}],
      [
        b.DatabaseGetFullAccounts,
        { names_or_ids: ['committee-account'], subscribe: false },
      ],
      [
        b.DatabaseGetKeyReferences,
        { keys: ['BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV'] },
      ],
      [
        b.DatabaseGetAssets,
        { asset_symbols_or_ids: ['BTS'], subscribe: false },
      ],
      [b.DatabaseLookupAssetSymbols, { symbols_or_ids: ['BTS'] }],
      [b.DatabaseListAssets, { lower_bound_symbol: '', limit: 5 }],
      [
        b.DatabaseLookupAccounts,
        { lower_bound_name: 'committee', limit: 5, subscribe: false },
      ],
      [b.DatabaseGetTicker, { base: '1.3.0', quote: '1.3.121' }],
      [b.HistoryGetFillOrderHistory, { a: '1.3.0', b: '1.3.121', limit: 5 }],
      [
        b.HistoryGetMarketHistory,
        {
          a: '1.3.0',
          b: '1.3.121',
          bucket_seconds: 60,
          start: new Date(Date.now() - 86400000).toISOString().slice(0, 19),
          end: new Date().toISOString().slice(0, 19),
        },
      ],
    ])
      await call(descriptor, params);
    const optional = async (descriptor, params, verify = () => {}) => {
      try {
        return await call(descriptor, params, verify);
      } catch (error) {
        const denied = /Access denied|not enabled|not available/.test(
          error.message,
        );
        report.checks.push({
          method: descriptor.api + '.' + descriptor.method,
          status: denied ? 'unavailable_on_node' : 'failed',
          reason: error.message.slice(0, 200),
        });
        return undefined;
      }
    };
    const groups = await optional(b.OrdersGetTrackedGroups, {});
    await optional(b.OrdersGetGroupedLimitOrders, {
      base_asset: '1.3.0',
      quote_asset: '1.3.121',
      group: groups?.[0] ?? 10,
      start: null,
      limit: 5,
    });
    const blind = new Uint8Array(32);
    blind[31] = 1;
    const nonce = new Uint8Array(32).fill(2);
    const commit = await optional(b.CryptoBlind, { blind, value: 7n }, (v) =>
      expect(v.length === 33, 'Commit length'),
    );
    await optional(
      b.CryptoBlindSum,
      { blinds_in: [blind, blind], non_neg: 1 },
      (v) =>
        expect(
          v.every((x) => x === 0),
          'Blind sum',
        ),
    );
    const commitHex = bytesToHex(commit ?? new Uint8Array(33));
    await optional(
      b.CryptoVerifySum,
      { commits_in: [commitHex], neg_commits_in: [commitHex], excess: 0n },
      (v) => expect(v === true, 'Commit sum'),
    );
    const proof = await optional(
      b.CryptoRangeProofSign,
      {
        min_value: 0n,
        commit: commitHex,
        commit_blind: blind,
        nonce,
        base10_exp: 0,
        min_bits: 8,
        actual_value: 7n,
      },
      (v) => expect(v.length > 0, 'Proof'),
    );
    await optional(
      b.CryptoVerifyRange,
      {
        commit: commit ?? new Uint8Array(33),
        proof: proof ?? new Uint8Array(),
      },
      (v) => expect(v.success && v.min_val <= 7n && v.max_val >= 7n, 'Range'),
    );
    await optional(
      b.CryptoVerifyRangeProofRewind,
      {
        nonce,
        commit: commit ?? new Uint8Array(33),
        proof: proof ?? new Uint8Array(),
      },
      (v) => expect(v.success && v.value_out === 7n, 'Rewind'),
    );
    await optional(
      b.CryptoRangeGetInfo,
      { proof: proof ?? new Uint8Array() },
      (v) => expect(v.min_value <= 7n && v.max_value >= 7n, 'Info'),
    );
    report.summary = Object.fromEntries(
      [...new Set(report.checks.map((c) => c.status))].map((status) => [
        status,
        report.checks.filter((c) => c.status === status).length,
      ]),
    );
    return report;
  } finally {
    client.close();
    report.finishedAt = new Date().toISOString();
  }
}
