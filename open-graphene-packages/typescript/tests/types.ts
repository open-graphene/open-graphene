import {
  AccountId,
  AssetId,
  FeeGrantId,
  operation,
  DatabaseGetObjects,
  type TransferOperation,
} from '../graphene-chain-mirrornet/graphene-chain-mirrornet-bindings/dist/index.js';
import { AccountId as BitSharesAccountId } from '../graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';
const account = AccountId('1.2.1');
const asset = AssetId('1.3.0');
const transfer: TransferOperation = {
  from: account,
  to: account,
  fee: { amount: 0n, asset_id: asset },
  amount: { amount: 1n, asset_id: asset },
  extensions: [],
};
const tag: 0 = operation.transfer(transfer)[0];
void tag;
// @ts-expect-error Asset IDs must not be accepted as account IDs.
const wrongAccount: AccountId = asset;
// @ts-expect-error IDs of different chains must not be interchangeable.
const wrongChain: AccountId = BitSharesAccountId('1.2.1');
const wrongAmount: TransferOperation = {
  ...transfer,
  // @ts-expect-error Generated amount fields must preserve 64-bit precision.
  amount: { amount: 1, asset_id: asset },
};
// @ts-expect-error Required RPC parameters may not be omitted.
DatabaseGetObjects.encodeParams({});
void [wrongAccount, wrongChain, wrongAmount];

import {
  BitSharesClient,
  type SignedTransfer as BitSharesSignedTransfer,
} from '../graphene-chain-bitshares/graphene-chain-bitshares-api/dist/index.js';
import {
  MirrornetClient,
  type PreparedTransfer,
  type PreparedTransaction,
} from '../graphene-chain-mirrornet/graphene-chain-mirrornet-api/dist/index.js';
import { Graphene } from '../graphene/dist/index.js';

function networkOptions(
  expectedChainId: string,
  prepared: PreparedTransfer | PreparedTransaction,
) {
  const options = {
    expectedChainId,
  };
  void MirrornetClient.connect('ws://fixture', options);
  void MirrornetClient.probeLatencies(['ws://fixture'], options);
  void Graphene.mirrornet('ws://fixture', options);
  void Graphene.connect('mirrornet', 'ws://fixture', options);
  // Without an expected network, a client pins the chain ID of its bindings.
  void MirrornetClient.connect('ws://fixture');
  void Graphene.connect('bitshares', 'ws://fixture');

  // @ts-expect-error The expected network is a chain ID string.
  void MirrornetClient.connect('ws://fixture', { expectedChainId: 1 });
  // @ts-expect-error A prepared transaction cannot change its signing network.
  prepared.chainId = expectedChainId;
}
void networkOptions;
function chainIsolation(
  bitshares: BitSharesClient,
  mirrornet: MirrornetClient,
  signed: BitSharesSignedTransfer,
) {
  void bitshares.broadcast(signed);
  // @ts-expect-error Signed transactions belong to their chain-specific API.
  void mirrornet.broadcast(signed);
}
void chainIsolation;

import {
  useAccount,
  useDatabaseGetAccounts,
  usePrepareTransfer,
  useSignTransaction,
  accountOptions,
  MirrornetProvider,
  MirrornetConnectionProvider,
  useMirrornetConnection,
} from '@open-graphene/chain-mirrornet-react';
import { useBitSharesConnection } from '@open-graphene/chain-bitshares-react';
import { QueryClient } from '@tanstack/react-query';
import type { SharedConnection } from '../graphene-transport/dist/index.js';
function reactTypes(client: MirrornetClient, other: BitSharesClient) {
  const selected: string | undefined = useAccount('alice', {
    live: true,
    select: (account) => account.name,
  }).data;
  const raw = useDatabaseGetAccounts({ account_names_or_ids: ['alice'] });
  const name: string | undefined = raw.data?.[0]?.name;
  const cached = new QueryClient().getQueryData(
    accountOptions(client, 'alice').queryKey,
  );
  const cachedName: string | undefined = cached?.name;
  const connection: SharedConnection = useMirrornetConnection();
  const sameType: SharedConnection = useBitSharesConnection();
  void MirrornetConnectionProvider({ connection, children: null });
  // @ts-expect-error The connection provider requires a connection.
  void MirrornetConnectionProvider({ children: null });
  // @ts-expect-error Generated RPC reads without a mapped subscription must reject live.
  useDatabaseGetAccounts({ account_names_or_ids: ['alice'] }, { live: true });
  // @ts-expect-error Required RPC parameters remain required.
  useDatabaseGetAccounts({});
  // @ts-expect-error Chain-specific query helpers cannot accept the other SDK client.
  accountOptions(other, 'alice');
  // @ts-expect-error Provider client types preserve chain isolation.
  MirrornetProvider({ client: other });
  const transfer = usePrepareTransfer();
  // @ts-expect-error Monetary values are exact decimal strings or bigint, not number.
  transfer.mutate({ from: 'a', to: 'b', amount: 1, maxFee: 1n });
  // @ts-expect-error A private key string is not a Signer and must not become a mutation variable.
  useSignTransaction('wif');
  void [selected, name, cachedName, sameType];
}
void reactTypes;

import {
  usePrepareDataRoomCreate,
  usePrepareTransferOperation,
  prepareDataRoomCreate,
  useCryptoBlind,
  useNetworkBroadcastBroadcastTransaction,
} from '@open-graphene/chain-mirrornet-react';
import { usePrepareTransferOperation as useBitSharesPrepareTransferOperation } from '@open-graphene/chain-bitshares-react';
function generatedMutationTypes(client: MirrornetClient) {
  const room = usePrepareDataRoomCreate({ maxFee: 300000n });
  room.mutate({
    payer: [1, { grant: FeeGrantId('1.29.1'), sponsor: AccountId('1.2.100') }],
    caller: [0, AccountId('1.2.101')],
    owner: AccountId('1.2.100'),
    name: 'room',
    description: '',
    caller_permissions: 15,
  });
  room.mutate({
    payer: [0, AccountId('1.2.100')],
    caller: [0, AccountId('1.2.100')],
    // @ts-expect-error Operation owner must be an account, not an asset ID.
    owner: AssetId('1.3.0'),
    name: 'room',
    description: '',
    caller_permissions: 15,
  });
  room.mutate({
    // @ts-expect-error A grant payer names its grant by a fee grant ID.
    payer: [1, { grant: AccountId('1.2.1'), sponsor: AccountId('1.2.100') }],
    caller: [0, AccountId('1.2.101')],
    owner: AccountId('1.2.100'),
    name: 'room',
    description: '',
    caller_permissions: 15,
  });
  // @ts-expect-error Required operation fields cannot be omitted.
  room.mutate({ name: 'room' });
  const raw = usePrepareTransferOperation();
  raw.mutate({
    from: account,
    to: account,
    amount: { amount: 1n, asset_id: asset },
  });
  // @ts-expect-error Generated transfer amounts are raw bigint asset amounts.
  raw.mutate({ from: account, to: account, amount: '0.01' });
  useBitSharesPrepareTransferOperation().mutate({
    // @ts-expect-error Operation hooks preserve chain-specific sender IDs.
    from: account,
    // @ts-expect-error Operation hooks preserve chain-specific recipient IDs.
    to: account,
    // @ts-expect-error Operation hooks preserve chain-specific asset IDs.
    amount: { amount: 1n, asset_id: asset },
  });
  useCryptoBlind().mutate({ blind: new Uint8Array(32), value: 1n });
  // @ts-expect-error Broadcast takes a checked typed signed transaction, not arbitrary JSON.
  useNetworkBroadcastBroadcastTransaction().mutate({ trx: 'raw JSON' });
  void prepareDataRoomCreate(client, {
    payer: [0, account],
    caller: [0, account],
    owner: account,
    name: 'room',
    description: '',
    caller_permissions: 15,
  });
}
void generatedMutationTypes;
