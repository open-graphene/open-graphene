import { adapter } from './context.js';
import type { SwaplockClient, TransferRequest, PreparedTransfer, PreparedTransaction, SignedTransfer, TransactionOptions } from '@open-graphene/chain-swaplock-api';
import type { Operation, AssetId } from '@open-graphene/chain-swaplock-bindings';
import type { Signer } from '@open-graphene/fc/signing';
import type { LiveReadOptions } from '@open-graphene/react-core';
export * from './generated/index.js';
export const SwaplockProvider = adapter.Provider;
export const useSwaplockClient = adapter.useClient;
type Account = Awaited<ReturnType<SwaplockClient['database']['account']>>;
type Balances = Awaited<ReturnType<SwaplockClient['database']['accountBalances']>>;
type Orders = Awaited<ReturnType<SwaplockClient['database']['accountOrders']>>;
type History = Awaited<ReturnType<SwaplockClient['history']['accountHistory']>>;
type Asset = Awaited<ReturnType<SwaplockClient['database']['asset']>>;
type Head = Awaited<ReturnType<SwaplockClient['database']['getDynamicGlobalProperties']>>;

export function accountOptions(client: SwaplockClient, nameOrId: string, scope?: string) {
  return adapter.options(client, 'account', nameOrId, (c, id) => c.database.account(id), scope);
}
export function useAccount<D = Account>(nameOrId: string, options: LiveReadOptions<Account, D> = {}) {
  return adapter.useRead('account', nameOrId, (c, id) => c.database.account(id), options, (c, id) => c.database.watchAccount(id));
}
export function accountBalancesOptions(client: SwaplockClient, nameOrId: string, assets: readonly AssetId[] = [], scope?: string) {
  return adapter.options(client, 'balances', { nameOrId, assets }, (c, p) => c.database.accountBalances(p.nameOrId, p.assets), scope);
}
export function useAccountBalances<D = Balances>(nameOrId: string, options: LiveReadOptions<Balances, D> & { readonly assets?: readonly AssetId[] } = {}) {
  const { assets = [], ...settings } = options;
  return adapter.useRead('balances', { nameOrId, assets }, (c, p) => c.database.accountBalances(p.nameOrId, p.assets), settings, (c, p) => c.database.watchBalances(p.nameOrId, p.assets));
}
export function accountOrdersOptions(client: SwaplockClient, nameOrId: string, scope?: string) {
  return adapter.options(client, 'orders', nameOrId, (c, id) => c.database.accountOrders(id), scope);
}
export function useAccountOrders<D = Orders>(nameOrId: string, options: LiveReadOptions<Orders, D> = {}) {
  return adapter.useRead('orders', nameOrId, (c, id) => c.database.accountOrders(id), options, (c, id) => c.database.watchAccountOrders(id));
}
export function accountHistoryOptions(client: SwaplockClient, nameOrId: string, limit = 20, offset = 0, scope?: string) {
  return adapter.options(client, 'history', { nameOrId, limit, offset }, (c, p) => c.history.accountHistory(p.nameOrId, p.limit, p.offset), scope);
}
export function useAccountHistory<D = History>(nameOrId: string, options: LiveReadOptions<History, D> & { readonly limit?: number; readonly offset?: number } = {}) {
  const { limit = 20, offset = 0, ...settings } = options;
  return adapter.useRead('history', { nameOrId, limit, offset }, (c, p) => c.history.accountHistory(p.nameOrId, p.limit, p.offset), settings, (c, p) => c.history.watchAccountHistory(p.nameOrId, p.limit, p.offset));
}
export function assetOptions(client: SwaplockClient, symbolOrId: string, scope?: string) {
  return adapter.options(client, 'asset', symbolOrId, (c, id) => c.database.asset(id), scope);
}
export function useAsset<D = Asset>(symbolOrId: string, options: LiveReadOptions<Asset, D> = {}) {
  return adapter.useRead('asset', symbolOrId, (c, id) => c.database.asset(id), options, (c, id) => c.database.watchAsset(id));
}
export function dynamicGlobalPropertiesOptions(client: SwaplockClient, scope?: string) {
  return adapter.options(client, 'head', null, c => c.database.getDynamicGlobalProperties({}), scope);
}
export function useDynamicGlobalProperties<D = Head>(options: LiveReadOptions<Head, D> = {}) {
  return adapter.useRead('head', null, c => c.database.getDynamicGlobalProperties({}), options, c => c.database.watchDynamicGlobalProperties());
}
export function usePrepareTransfer() {
  return adapter.useAction((client, request: TransferRequest) => client.prepareTransfer(request));
}
export function usePrepareOperations() {
  return adapter.useAction((client, request: { readonly operations: readonly Operation[]; readonly options?: TransactionOptions }) => client.prepareOperations(request.operations, request.options));
}
/** Signers are captured by the callback, never stored in mutation variables. */
export function useSignTransaction(signers: Signer | readonly Signer[]) {
  return adapter.useAction((_client, prepared: PreparedTransaction | PreparedTransfer) => prepared.sign(signers));
}
export function useBroadcastTransaction() {
  return adapter.useAction((client, signed: SignedTransfer) => client.broadcast(signed));
}
/** Invalidate this client's reads after inclusion, not merely after submission. */
export function useWaitForInclusion() {
  return adapter.useAction((client, signed: SignedTransfer) => client.waitForInclusion(signed), true);
}

import { DataRoomId, ContentCardId } from '@open-graphene/chain-swaplock-bindings';
import { objectId } from '@open-graphene/primitives';
type Room = Awaited<ReturnType<SwaplockClient['database']['getDataRoomById']>>;
type Access = Awaited<ReturnType<SwaplockClient['database']['getDataRoomAccessState']>>;
type Card = Awaited<ReturnType<SwaplockClient['database']['getContentCardById']>>;
// Room/member/card updates can touch multiple objects. A block notice triggers a full resnapshot.
function onBlocks<T>(client: SwaplockClient, fetch: () => Promise<T>) {
  return client.queries.watchQuery(fetch, () => client.database.getObjects({ ids: [objectId('2.1.0')], subscribe: true }));
}
export function roomOptions(client: SwaplockClient, room: DataRoomId, scope?: string) {
  return adapter.options(client, 'room', room, (c, id) => c.database.getDataRoomById({ room_id: id }), scope);
}
export function useRoom<D = Room>(room: DataRoomId, options: LiveReadOptions<Room, D> = {}) {
  return adapter.useRead('room', room, (c, id) => c.database.getDataRoomById({ room_id: id }), options,
    (c, id) => onBlocks(c, () => c.database.getDataRoomById({ room_id: id })));
}
export function roomAccessStateOptions(client: SwaplockClient, room: DataRoomId, scope?: string) {
  return adapter.options(client, 'room-access', room, (c, id) => c.database.getDataRoomAccessState({ room_id: id }), scope);
}
export function useRoomAccessState<D = Access>(room: DataRoomId, options: LiveReadOptions<Access, D> = {}) {
  return adapter.useRead('room-access', room, (c, id) => c.database.getDataRoomAccessState({ room_id: id }), options,
    (c, id) => onBlocks(c, () => c.database.getDataRoomAccessState({ room_id: id })));
}
export function contentCardOptions(client: SwaplockClient, card: ContentCardId, scope?: string) {
  return adapter.options(client, 'card', card, (c, id) => c.database.getContentCardById({ content_id: id }), scope);
}
export function useContentCard<D = Card>(card: ContentCardId, options: LiveReadOptions<Card, D> = {}) {
  return adapter.useRead('card', card, (c, id) => c.database.getContentCardById({ content_id: id }), options,
    (c, id) => onBlocks(c, () => c.database.getContentCardById({ content_id: id })));
}
