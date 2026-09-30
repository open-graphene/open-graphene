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

import { useAccount, useDatabaseGetAccounts, usePrepareTransfer, useSignTransaction, accountOptions, SwaplockProvider, useRoom } from '../graphene-react/dist/swaplock.js';
import { QueryClient } from '@tanstack/react-query';
import { DataRoomId } from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
function reactTypes(client: SwaplockClient, other: BitSharesClient) {
  const selected: string | undefined = useAccount('alice', { live: true, select: account => account.name }).data;
  const raw = useDatabaseGetAccounts({ account_names_or_ids: ['alice'] });
  const name: string | undefined = raw.data?.[0]?.name;
  const cached = new QueryClient().getQueryData(accountOptions(client, 'alice').queryKey);
  const cachedName: string | undefined = cached?.name;
  useRoom(DataRoomId('1.23.1'), { live: true });
  // @ts-expect-error Generated RPC reads without a mapped subscription must reject live.
  useDatabaseGetAccounts({ account_names_or_ids: ['alice'] }, { live: true });
  // @ts-expect-error Required RPC parameters remain required.
  useDatabaseGetAccounts({});
  // @ts-expect-error Chain-specific query helpers cannot accept the other SDK client.
  accountOptions(other, 'alice');
  // @ts-expect-error Provider client types preserve chain isolation.
  SwaplockProvider({ client: other });
  // @ts-expect-error Room IDs are distinct from account IDs.
  useRoom(AccountId('1.2.1'));
  const transfer = usePrepareTransfer();
  // @ts-expect-error Monetary values are exact decimal strings or bigint, not number.
  transfer.mutate({ from: 'a', to: 'b', amount: 1, maxFee: 1n });
  // @ts-expect-error A private key string is not a Signer and must not become a mutation variable.
  useSignTransaction('wif');
  void [selected, name, cachedName];
}
void reactTypes;
