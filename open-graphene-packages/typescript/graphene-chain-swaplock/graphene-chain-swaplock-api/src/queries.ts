import * as b from '@open-graphene/chain-swaplock-bindings';
import { RpcSubscription } from '@open-graphene/transport';
import { vector, type WireValue } from '@open-graphene/codec';
import { objectId } from '@open-graphene/primitives';
import { type SwaplockClient, type SignedTransfer } from './index.js';

import { sendTransactionWithCallback } from './broadcast.js';

export class Queries {
  constructor(private readonly client: SwaplockClient) {}
  async account(nameOrId: string): Promise<b.AccountObject> {
    const [account] = await this.client.rpc.invoke(b.DatabaseGetAccounts, {
      account_names_or_ids: [nameOrId],
      subscribe: false,
    });
    if (!account) throw new Error('Account not found: ' + nameOrId);
    return account;
  }
  async asset(symbolOrId: string): Promise<b.ExtendedAssetObject> {
    const [asset] = await this.client.rpc.invoke(b.DatabaseGetAssets, {
      asset_symbols_or_ids: [symbolOrId],
      subscribe: false,
    });
    if (!asset) throw new Error('Asset not found: ' + symbolOrId);
    return asset;
  }
  async balances(account: string, assets: readonly b.AssetId[] = []) {
    return this.client.rpc.invoke(b.DatabaseGetAccountBalances, {
      account_name_or_id: account,
      assets,
    });
  }
  async accountOrders(account: string) {
    const result = await this.client.rpc.invoke(b.DatabaseGetFullAccounts, {
      names_or_ids: [account],
      subscribe: false,
    });
    if (!result.length) throw new Error('Account not found: ' + account);
    return result[0]![1].limit_orders;
  }
  async proposedTransactions(
    account: string,
  ): Promise<readonly b.ProposalObject[]> {
    return vector(b.ProposalObjectCodec).decode(
      await this.client.rpc.request('database', 'get_proposed_transactions', [
        account,
      ]),
    );
  }
  async accountHistory(account: string, limit = 20, offset = 0) {
    if (
      !Number.isSafeInteger(limit) ||
      !Number.isSafeInteger(offset) ||
      limit < 1 ||
      offset < 0 ||
      offset + limit > 98
    )
      throw new Error('History window must fit 1..98');
    const raw = await this.client.rpc.invoke(b.HistoryGetAccountHistory, {
      account_name_or_id: account,
      stop: b.OperationHistoryId('1.11.0'),
      limit: offset + limit + 1,
      start: b.OperationHistoryId('1.11.0'),
    });
    return {
      items: raw.slice(offset, offset + limit),
      limit,
      offset,
      nextOffset: raw.length > offset + limit ? offset + limit : null,
      hasMore: raw.length > offset + limit,
    };
  }
  async watchQuery<T>(
    fetch: () => Promise<T>,
    register: () => Promise<unknown>,
  ): Promise<RpcSubscription<T>> {
    const notices = await this.client.rpc.databaseNotices();
    const output = new RpcSubscription<T>(notices.callbackId, () =>
      notices.close(),
    );
    try {
      await register();
      output.push(await fetch());
      void (async () => {
        try {
          for await (const _ of notices) {
            output.push(await fetch());
          }
        } catch (e) {
          output.fail(e instanceof Error ? e : new Error(String(e)));
        } finally {
          output.close();
        }
      })();
      return output;
    } catch (e) {
      output.close();
      throw e;
    }
  }
  #registerAccount(account: string) {
    return this.client.rpc.invoke(b.DatabaseGetFullAccounts, {
      names_or_ids: [account],
      subscribe: true,
    });
  }
  watchAccount(account: string) {
    return this.watchQuery(
      () => this.account(account),
      () => this.#registerAccount(account),
    );
  }
  watchBalances(account: string, assets: readonly b.AssetId[] = []) {
    return this.watchQuery(
      () => this.balances(account, assets),
      () => this.#registerAccount(account),
    );
  }
  watchAccountOrders(account: string) {
    return this.watchQuery(
      () => this.accountOrders(account),
      () => this.#registerAccount(account),
    );
  }
  watchAccountHistory(account: string, limit = 20, offset = 0) {
    return this.watchQuery(
      () => this.accountHistory(account, limit, offset),
      () => this.#registerAccount(account),
    );
  }
  watchAsset(asset: string) {
    return this.watchQuery(
      () => this.asset(asset),
      async () => {
        const a = await this.asset(asset);
        return this.client.rpc.invoke(b.DatabaseGetObjects, {
          ids: [objectId(a.id)],
          subscribe: true,
        });
      },
    );
  }
  watchDynamicGlobalProperties() {
    return this.watchQuery(
      () => this.client.rpc.invoke(b.DatabaseGetDynamicGlobalProperties, {}),
      () =>
        this.client.rpc.invoke(b.DatabaseGetObjects, {
          ids: [objectId('2.1.0')],
          subscribe: true,
        }),
    );
  }
  async subscribeMarket(base: b.AssetId, quote: b.AssetId) {
    if (base === quote) throw new Error('Market assets must differ');
    const stream = await this.client.rpc.subscribe(
      'database',
      'subscribe_to_market',
      (id) => [id, base, quote],
    );
    const output = new RpcSubscription<WireValue>(stream.callbackId, () => {
      stream.close();
      void this.client.rpc
        .request('database', 'unsubscribe_from_market', [base, quote])
        .catch(() => {});
    });
    void (async () => {
      try {
        for await (const value of stream) output.push(value);
      } catch (e) {
        output.fail(e instanceof Error ? e : new Error(String(e)));
      } finally {
        output.close();
      }
    })();
    return output;
  }
  async broadcastWithCallback(transaction: SignedTransfer) {
    return sendTransactionWithCallback(this.client.rpc, transaction);
  }
}
