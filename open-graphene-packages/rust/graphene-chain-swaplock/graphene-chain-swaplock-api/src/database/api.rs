use graphene_chain_swaplock_bindings::generated::{
    AccountObject, Asset, AssetObject, ChainPropertyObject, DynamicGlobalPropertyObject,
    GlobalPropertyObject, LimitOrderObject,
};
use open_graphene_transport::GrapheneSession;
use serde_json::Value;

use crate::SwaplockApiError;

use super::account_balances::AccountBalancesRequest;
use super::account_balances_by_id::{AccountBalancesByIdRequest, get_account_balances_by_id};
use super::account_by_id::{AccountByIdRequest, get_account_by_id};
use super::account_by_name::{AccountByNameRequest, get_account_by_name};
use super::account_orders::AccountOrdersRequest;
use super::account_orders_by_id::{AccountOrdersByIdRequest, get_account_orders_by_id};
use super::accounts::AccountsRequest;
use super::asset_by_id::{AssetByIdRequest, get_asset_by_id};
use super::asset_by_symbol::{AssetBySymbolRequest, get_asset_by_symbol};
use super::chain_id::{ChainIdRequest, get_chain_id};
use super::chain_properties::{ChainPropertiesRequest, get_chain_properties};
use super::dynamic_global_properties::{
    DynamicGlobalPropertiesRequest, DynamicGlobalPropertiesSubscription,
    get_dynamic_global_properties,
};
use super::get_block::GetBlockRequest;
use super::get_limit_orders::{DEFAULT_GET_LIMIT_ORDERS_LIMIT, GetLimitOrdersRequest};
use super::get_objects::{GetObjectsRequest, get_objects_request};
use super::global_properties::{GlobalPropertiesRequest, get_global_properties};
use super::list_assets::{DEFAULT_LIST_ASSETS_LIMIT, ListAssetsRequest};
use super::lookup_accounts::{DEFAULT_LOOKUP_ACCOUNTS_LIMIT, LookupAccountsRequest};
use super::string_list::IntoStringList;

pub struct DatabaseApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl<'session> DatabaseApi<'session> {
    pub fn account_balances<A, L>(
        self,
        account_name: A,
        asset_symbols: L,
    ) -> AccountBalancesRequest<'session>
    where
        A: Into<String>,
        L: IntoStringList,
    {
        AccountBalancesRequest {
            session: self.session,
            account_name: account_name.into(),
            asset_symbols: asset_symbols.into_string_list(),
        }
    }

    pub fn account_balances_by_id<A, L>(
        self,
        account_id: A,
        asset_ids: L,
    ) -> AccountBalancesByIdRequest<'session>
    where
        A: Into<String>,
        L: IntoStringList,
    {
        AccountBalancesByIdRequest {
            session: self.session,
            account_id: account_id.into(),
            asset_ids: asset_ids.into_string_list(),
        }
    }

    pub fn account_orders<S>(self, account_name: S) -> AccountOrdersRequest<'session>
    where
        S: Into<String>,
    {
        AccountOrdersRequest {
            session: self.session,
            account_name: account_name.into(),
        }
    }

    pub fn account_orders_by_id<S>(self, account_id: S) -> AccountOrdersByIdRequest<'session>
    where
        S: Into<String>,
    {
        AccountOrdersByIdRequest {
            session: self.session,
            account_id: account_id.into(),
        }
    }

    pub fn account_by_id<S>(self, account_id: S) -> AccountByIdRequest<'session>
    where
        S: Into<String>,
    {
        AccountByIdRequest {
            session: self.session,
            account_id: account_id.into(),
        }
    }

    pub fn account_by_name<S>(self, account_name: S) -> AccountByNameRequest<'session>
    where
        S: Into<String>,
    {
        AccountByNameRequest {
            session: self.session,
            account_name: account_name.into(),
        }
    }

    pub fn accounts<I, S>(self, names_or_ids: I) -> AccountsRequest<'session>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        AccountsRequest {
            session: self.session,
            names_or_ids: names_or_ids.into_iter().map(Into::into).collect(),
        }
    }

    pub fn asset_by_id<S>(self, asset_id: S) -> AssetByIdRequest<'session>
    where
        S: Into<String>,
    {
        AssetByIdRequest {
            session: self.session,
            asset_id: asset_id.into(),
        }
    }

    pub fn asset_by_symbol<S>(self, asset_symbol: S) -> AssetBySymbolRequest<'session>
    where
        S: Into<String>,
    {
        AssetBySymbolRequest {
            session: self.session,
            asset_symbol: asset_symbol.into(),
        }
    }

    pub fn chain_id(self) -> ChainIdRequest<'session> {
        ChainIdRequest {
            session: self.session,
        }
    }

    pub fn chain_properties(self) -> ChainPropertiesRequest<'session> {
        ChainPropertiesRequest {
            session: self.session,
        }
    }

    pub fn dynamic_global_properties(self) -> DynamicGlobalPropertiesRequest<'session> {
        DynamicGlobalPropertiesRequest {
            session: self.session,
        }
    }

    pub fn global_properties(self) -> GlobalPropertiesRequest<'session> {
        GlobalPropertiesRequest {
            session: self.session,
        }
    }

    /// List assets in symbol order, starting from `lower_bound` (`""` for the start).
    pub fn list_assets<S>(self, lower_bound: S) -> ListAssetsRequest<'session>
    where
        S: Into<String>,
    {
        ListAssetsRequest {
            session: self.session,
            lower_bound: lower_bound.into(),
            limit: DEFAULT_LIST_ASSETS_LIMIT,
        }
    }

    /// Look up account `(name, id)` pairs in name order, starting from `lower_bound`.
    pub fn lookup_accounts<S>(self, lower_bound: S) -> LookupAccountsRequest<'session>
    where
        S: Into<String>,
    {
        LookupAccountsRequest {
            session: self.session,
            lower_bound: lower_bound.into(),
            limit: DEFAULT_LOOKUP_ACCOUNTS_LIMIT,
        }
    }

    /// The raw order book for the `base`/`quote` market (asset ids like `1.3.0`).
    pub fn limit_orders<B, Q>(self, base: B, quote: Q) -> GetLimitOrdersRequest<'session>
    where
        B: Into<String>,
        Q: Into<String>,
    {
        GetLimitOrdersRequest {
            session: self.session,
            base: base.into(),
            quote: quote.into(),
            limit: DEFAULT_GET_LIMIT_ORDERS_LIMIT,
        }
    }

    /// Fetch a produced block by height (`None` if the chain has not reached it yet).
    pub fn block(self, block_num: u32) -> GetBlockRequest<'session> {
        GetBlockRequest {
            session: self.session,
            block_num,
        }
    }

    /// Fetch any chain objects by id as raw JSON, e.g. `["2.1.0", "1.3.0"]`. The generic getter
    /// behind the typed ones; results keep the id order and unknown ids come back as `null`.
    pub fn objects<L>(self, ids: L) -> GetObjectsRequest<'session>
    where
        L: IntoStringList,
    {
        get_objects_request(self.session, ids)
    }

    pub async fn get_account_balances<L>(
        &mut self,
        account_name: &str,
        asset_symbols: L,
    ) -> Result<Vec<Asset>, SwaplockApiError>
    where
        L: IntoStringList,
    {
        let account = get_account_by_name(self.session, account_name).await?;
        let mut asset_ids = Vec::new();
        for symbol in asset_symbols.into_string_list() {
            let asset = get_asset_by_symbol(self.session, &symbol).await?;
            asset_ids.push(asset.id.0);
        }
        get_account_balances_by_id(self.session, &account.id.0, asset_ids).await
    }

    pub async fn get_account_balances_by_id<L>(
        &mut self,
        account_id: &str,
        asset_ids: L,
    ) -> Result<Vec<Asset>, SwaplockApiError>
    where
        L: IntoStringList,
    {
        get_account_balances_by_id(self.session, account_id, asset_ids.into_string_list()).await
    }

    pub async fn get_account_orders(
        &mut self,
        account_name: &str,
    ) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        let account = get_account_by_name(self.session, account_name).await?;
        get_account_orders_by_id(self.session, &account.id.0).await
    }

    pub async fn get_account_orders_by_id(
        &mut self,
        account_id: &str,
    ) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        get_account_orders_by_id(self.session, account_id).await
    }

    pub async fn get_limit_orders(
        &mut self,
        base: &str,
        quote: &str,
    ) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        GetLimitOrdersRequest {
            session: &mut *self.session,
            base: base.to_string(),
            quote: quote.to_string(),
            limit: DEFAULT_GET_LIMIT_ORDERS_LIMIT,
        }
        .get()
        .await
    }

    pub async fn get_block(&mut self, block_num: u32) -> Result<Option<Value>, SwaplockApiError> {
        GetBlockRequest {
            session: &mut *self.session,
            block_num,
        }
        .get()
        .await
    }

    pub async fn get_objects<L>(&mut self, ids: L) -> Result<Vec<Value>, SwaplockApiError>
    where
        L: IntoStringList,
    {
        get_objects_request(self.session, ids).get().await
    }

    pub async fn get_account_by_name(
        &mut self,
        account_name: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_name(self.session, account_name).await
    }

    pub async fn get_account_by_id(
        &mut self,
        account_id: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_id(self.session, account_id).await
    }

    pub async fn get_asset_by_symbol(
        &mut self,
        asset_symbol: &str,
    ) -> Result<AssetObject, SwaplockApiError> {
        get_asset_by_symbol(self.session, asset_symbol).await
    }

    pub async fn get_asset_by_id(
        &mut self,
        asset_id: &str,
    ) -> Result<AssetObject, SwaplockApiError> {
        get_asset_by_id(self.session, asset_id).await
    }

    pub async fn get_chain_id(&mut self) -> Result<String, SwaplockApiError> {
        get_chain_id(self.session).await
    }

    pub async fn get_chain_properties(&mut self) -> Result<ChainPropertyObject, SwaplockApiError> {
        get_chain_properties(self.session).await
    }

    pub async fn get_dynamic_global_properties(
        &mut self,
    ) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        get_dynamic_global_properties(self.session).await
    }

    pub async fn get_global_properties(
        &mut self,
    ) -> Result<GlobalPropertyObject, SwaplockApiError> {
        get_global_properties(self.session).await
    }

    pub async fn subscribe_dynamic_global_properties(
        self,
    ) -> Result<DynamicGlobalPropertiesSubscription<'session>, SwaplockApiError> {
        self.dynamic_global_properties().subscribe().await
    }
}
