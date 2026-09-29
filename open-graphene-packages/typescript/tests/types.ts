import { AccountId, AssetId, operation, DatabaseGetObjects, type TransferOperation } from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { AccountId as BitSharesAccountId } from '../graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';
const account = AccountId('1.2.1');
const asset = AssetId('1.3.0');
const transfer: TransferOperation = { from: account, to: account, fee: { amount: 0n, asset_id: asset }, amount: { amount: 1n, asset_id: asset }, extensions: [] };
const tag: 0 = operation.transfer(transfer)[0];
void tag;
// @ts-expect-error Asset IDs must not be accepted as account IDs.
const wrongAccount: AccountId = asset;
// @ts-expect-error IDs of different chains must not be interchangeable.
const wrongChain: AccountId = BitSharesAccountId('1.2.1');
// @ts-expect-error Generated amount fields must preserve 64-bit precision.
const wrongAmount: TransferOperation = { ...transfer, amount: { amount: 1, asset_id: asset } };
// @ts-expect-error Required RPC parameters may not be omitted.
DatabaseGetObjects.encodeParams({});
void [wrongAccount, wrongChain, wrongAmount];

import { BitSharesClient, type SignedTransfer as BitSharesSignedTransfer } from '../graphene-chain-bitshares/graphene-chain-bitshares-api/dist/index.js';
import type { SwaplockClient } from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
function chainIsolation(bitshares: BitSharesClient, swaplock: SwaplockClient, signed: BitSharesSignedTransfer) {
  void bitshares.broadcast(signed);
  // @ts-expect-error Signed transactions belong to their chain-specific API.
  void swaplock.broadcast(signed);
}
void chainIsolation;
