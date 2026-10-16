import test from 'node:test';
import assert from 'node:assert/strict';
import * as b from '../graphene-chain-rsquared/graphene-chain-rsquared-bindings/dist/index.js';
import * as react from '../graphene-chain-rsquared/graphene-chain-rsquared-react/dist/index.js';
import { bytesToHex } from '../graphene-primitives/dist/index.js';

// operations.hpp and FC_REFLECT(personal_data_create_operation) in pinned R-Squared-core.
const personalData = {
  fee: { amount: 0n, asset_id: b.AssetId('1.3.0') },
  subject_account: b.AccountId('1.2.1'),
  operator_account: b.AccountId('1.2.2'),
  url: 'a',
  hash: 'b',
  storage_data: 'c',
};

test('R-Squared encodes personal data with its own tag and reflected field order', () => {
  const operation = b.operation.personal_data_create(personalData);
  assert.equal(operation[0], 39);
  assert.equal(
    bytesToHex(b.encodeOperation(operation)),
    '270000000000000000000102016101620163',
  );
  assert.deepEqual(
    b.OperationCodec.decode(b.OperationCodec.encode(operation)),
    operation,
  );
  assert.equal(
    b.operation.limit_order_cancel({
      fee: personalData.fee,
      fee_paying_account: personalData.subject_account,
      order: b.LimitOrderId('1.7.1'),
      extensions: [],
    })[0],
    55,
  );
});

test('R-Squared fee schedule decodes legacy fee_parameters_type and rejects unknown tags', () => {
  const fee = b.FeeParametersCodec.decode([
    39,
    { fee: '9007199254740993', price_per_kbyte: 10 },
  ]);
  assert.equal(fee[1].fee, 9007199254740993n);
  assert.throws(() => b.OperationCodec.decode([64, {}]));
  assert.throws(() => b.FeeParametersCodec.decode([64, {}]));
});

test('R-Squared exposes domain RPCs and excludes virtual operations from React preparation', () => {
  assert.equal(b.CHAIN.publicKeyPrefix, 'RQRX');
  assert.equal(
    b.CHAIN.chainId,
    'a89f8a1cd2a699e5c521b87cc6210198ed0aad9e2a483322c6db2c391b278f64',
  );
  assert.equal(react.generatedPrepareOperations.length, 59);
  assert.ok(react.generatedPrepareOperations.includes('content_card_create'));
  assert.ok(react.generatedPrepareOperations.includes('permission_create'));
  assert.ok(!react.generatedPrepareOperations.includes('fill_order'));
  assert.ok(react.generatedQueryMethods.includes('database.get_personal_data'));
  assert.ok(react.generatedQueryMethods.includes('database.get_content_cards'));
  assert.ok(react.generatedQueryMethods.includes('database.get_permissions'));
  assert.deepEqual(react.generatedRpcMutations, [
    'network_broadcast.broadcast_transaction',
  ]);
});

test('R-Squared preparation forwards personal data and fee limits to the client', async () => {
  const calls = [];
  const client = {
    async prepareOperations(operations, options) {
      calls.push({ operations, options });
      return 'prepared';
    },
  };
  const { fee, ...input } = personalData;
  const result = await react.preparePersonalDataCreate(client, input, {
    maxFee: 10n,
  });

  assert.equal(result, 'prepared');
  assert.deepEqual(calls, [
    {
      operations: [b.operation.personal_data_create(personalData)],
      options: { maxFee: 10n },
    },
  ]);
});
