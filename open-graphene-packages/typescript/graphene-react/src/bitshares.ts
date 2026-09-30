import { adapter } from './bitshares-context.js';
import type { BitSharesClient, TransferRequest, PreparedTransfer, PreparedTransaction, SignedTransfer, TransactionOptions } from '@open-graphene/chain-bitshares-api';
import type { Operation, AssetId } from '@open-graphene/chain-bitshares-bindings';
import type { Signer } from '@open-graphene/fc/signing';
import type { LiveReadOptions } from './adapter.js';
export * from './generated/bitshares.js';
export const BitSharesProvider = adapter.Provider;
export const useBitSharesClient = adapter.useClient;
type Account = Awaited<ReturnType<BitSharesClient['database']['account']>>;
type Balances = Awaited<ReturnType<BitSharesClient['database']['accountBalances']>>;
type Orders = Awaited<ReturnType<BitSharesClient['database']['accountOrders']>>;
type History = Awaited<ReturnType<BitSharesClient['history']['accountHistory']>>;
type Asset = Awaited<ReturnType<BitSharesClient['database']['asset']>>;
type Head = Awaited<ReturnType<BitSharesClient['database']['getDynamicGlobalProperties']>>;

export function accountOptions(client: BitSharesClient, nameOrId: string, scope?: string) {
  return adapter.options(client, 'account', nameOrId, (c, id) => c.database.account(id), scope);
}
export function useAccount<D = Account>(nameOrId: string, options: LiveReadOptions<Account, D> = {}) {
  return adapter.useRead('account', nameOrId, (c, id) => c.database.account(id), options, (c, id) => c.database.watchAccount(id));
}
export function accountBalancesOptions(client: BitSharesClient, nameOrId: string, assets: readonly AssetId[] = [], scope?: string) {
  return adapter.options(client, 'balances', { nameOrId, assets }, (c, p) => c.database.accountBalances(p.nameOrId, p.assets), scope);
}
export function useAccountBalances<D = Balances>(nameOrId: string, options: LiveReadOptions<Balances, D> & { readonly assets?: readonly AssetId[] } = {}) {
  const { assets = [], ...settings } = options;
  return adapter.useRead('balances', { nameOrId, assets }, (c, p) => c.database.accountBalances(p.nameOrId, p.assets), settings, (c, p) => c.database.watchBalances(p.nameOrId, p.assets));
}
export function accountOrdersOptions(client: BitSharesClient, nameOrId: string, scope?: string) {
  return adapter.options(client, 'orders', nameOrId, (c, id) => c.database.accountOrders(id), scope);
}
export function useAccountOrders<D = Orders>(nameOrId: string, options: LiveReadOptions<Orders, D> = {}) {
  return adapter.useRead('orders', nameOrId, (c, id) => c.database.accountOrders(id), options, (c, id) => c.database.watchAccountOrders(id));
}
export function accountHistoryOptions(client: BitSharesClient, nameOrId: string, limit = 20, offset = 0, scope?: string) {
  return adapter.options(client, 'history', { nameOrId, limit, offset }, (c, p) => c.history.accountHistory(p.nameOrId, p.limit, p.offset), scope);
}
export function useAccountHistory<D = History>(nameOrId: string, options: LiveReadOptions<History, D> & { readonly limit?: number; readonly offset?: number } = {}) {
  const { limit = 20, offset = 0, ...settings } = options;
  return adapter.useRead('history', { nameOrId, limit, offset }, (c, p) => c.history.accountHistory(p.nameOrId, p.limit, p.offset), settings, (c, p) => c.history.watchAccountHistory(p.nameOrId, p.limit, p.offset));
}
export function assetOptions(client: BitSharesClient, symbolOrId: string, scope?: string) {
  return adapter.options(client, 'asset', symbolOrId, (c, id) => c.database.asset(id), scope);
}
export function useAsset<D = Asset>(symbolOrId: string, options: LiveReadOptions<Asset, D> = {}) {
  return adapter.useRead('asset', symbolOrId, (c, id) => c.database.asset(id), options, (c, id) => c.database.watchAsset(id));
}
export function dynamicGlobalPropertiesOptions(client: BitSharesClient, scope?: string) {
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
