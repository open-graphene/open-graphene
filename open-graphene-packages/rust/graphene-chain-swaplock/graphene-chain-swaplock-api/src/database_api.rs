use graphene_chain_swaplock_bindings::generated::{
    AccountBalanceObject, AccountObject, Asset, AssetObject, ChainPropertyObject,
    DynamicGlobalPropertyObject, GlobalPropertyObject, LimitOrderObject,
};
use open_graphene_transport::{GrapheneSession, JsonRpcInbound, parse_chain_id};
use serde_json::{Value, json};

use crate::SwaplockApiError;

const ACCOUNT_BALANCES_CALLBACK_ID: u64 = 4;
const ACCOUNT_ORDERS_CALLBACK_ID: u64 = 5;
const ACCOUNT_CALLBACK_ID: u64 = 2;
const ASSET_CALLBACK_ID: u64 = 3;
const DYNAMIC_GLOBAL_PROPERTIES_ID: &str = "2.1.0";
const DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID: u64 = 1;

pub trait IntoStringList {
    fn into_string_list(self) -> Vec<String>;
}

impl<const N: usize> IntoStringList for [&str; N] {
    fn into_string_list(self) -> Vec<String> {
        self.into_iter().map(str::to_string).collect()
    }
}

impl IntoStringList for Vec<String> {
    fn into_string_list(self) -> Vec<String> {
        self
    }
}

impl IntoStringList for Vec<&str> {
    fn into_string_list(self) -> Vec<String> {
        self.into_iter().map(str::to_string).collect()
    }
}

impl IntoStringList for Vec<&String> {
    fn into_string_list(self) -> Vec<String> {
        self.into_iter().map(ToString::to_string).collect()
    }
}

pub struct DatabaseApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

pub struct AccountByIdRequest<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
}

pub struct AccountByNameRequest<'session> {
    session: &'session mut GrapheneSession,
    account_name: String,
}

pub struct AccountSubscription<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    initial: AccountObject,
}

pub struct AccountOrdersRequest<'session> {
    session: &'session mut GrapheneSession,
    account_name: String,
}

pub struct AccountOrdersByIdRequest<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
}

pub struct AccountOrdersSubscription<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    initial: Vec<LimitOrderObject>,
}

pub struct AccountBalancesRequest<'session> {
    session: &'session mut GrapheneSession,
    account_name: String,
    asset_symbols: Vec<String>,
}

pub struct AccountBalancesByIdRequest<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    asset_ids: Vec<String>,
}

pub struct AccountBalancesSubscription<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    asset_ids: Vec<String>,
    initial: Vec<AccountBalanceObject>,
}

pub struct AccountsRequest<'session> {
    session: &'session mut GrapheneSession,
    names_or_ids: Vec<String>,
}

pub struct AssetByIdRequest<'session> {
    session: &'session mut GrapheneSession,
    asset_id: String,
}

pub struct AssetBySymbolRequest<'session> {
    session: &'session mut GrapheneSession,
    asset_symbol: String,
}

pub struct AssetSubscription<'session> {
    session: &'session mut GrapheneSession,
    asset_id: String,
    initial: AssetObject,
}

pub struct ChainIdRequest<'session> {
    session: &'session mut GrapheneSession,
}

pub struct ChainPropertiesRequest<'session> {
    session: &'session mut GrapheneSession,
}

pub struct DynamicGlobalPropertiesRequest<'session> {
    session: &'session mut GrapheneSession,
}

pub struct DynamicGlobalPropertiesSubscription<'session> {
    session: &'session mut GrapheneSession,
    initial: DynamicGlobalPropertyObject,
}

pub struct GlobalPropertiesRequest<'session> {
    session: &'session mut GrapheneSession,
}

impl<'session> AccountOrdersRequest<'session> {
    pub async fn get(self) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        get_account_orders_by_id(self.session, &account.id.0).await
    }

    pub async fn subscribe(self) -> Result<AccountOrdersSubscription<'session>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        subscribe_account_orders_by_id(self.session, account.id.0).await
    }
}

impl<'session> AccountOrdersByIdRequest<'session> {
    pub async fn get(self) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        get_account_orders_by_id(self.session, &self.account_id).await
    }

    pub async fn subscribe(self) -> Result<AccountOrdersSubscription<'session>, SwaplockApiError> {
        subscribe_account_orders_by_id(self.session, self.account_id).await
    }
}

impl AccountOrdersSubscription<'_> {
    pub fn initial(&self) -> &[LimitOrderObject] {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice()?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != ACCOUNT_ORDERS_CALLBACK_ID {
                continue;
            }

            let updates = collect_account_order_objects("notice", &payload, &self.account_id)?;
            if !updates.is_empty() {
                return Ok(updates);
            }
        }
    }
}

impl<'session> AccountBalancesRequest<'session> {
    pub async fn get(self) -> Result<Vec<Asset>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        let mut asset_ids = Vec::with_capacity(self.asset_symbols.len());
        for symbol in self.asset_symbols {
            let asset = get_asset_by_symbol(self.session, &symbol).await?;
            asset_ids.push(asset.id.0);
        }

        get_account_balances_by_id(self.session, &account.id.0, asset_ids).await
    }

    pub async fn subscribe(
        self,
    ) -> Result<AccountBalancesSubscription<'session>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        let mut asset_ids = Vec::with_capacity(self.asset_symbols.len());
        for symbol in self.asset_symbols {
            let asset = get_asset_by_symbol(self.session, &symbol).await?;
            asset_ids.push(asset.id.0);
        }

        subscribe_account_balances_by_id(self.session, account.id.0, asset_ids).await
    }
}

impl AccountBalancesSubscription<'_> {
    pub fn initial(&self) -> &[AccountBalanceObject] {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<Vec<AccountBalanceObject>, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice()?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != ACCOUNT_BALANCES_CALLBACK_ID {
                continue;
            }

            let updates = collect_account_balance_objects(
                "notice",
                &payload,
                &self.account_id,
                &self.asset_ids,
            )?;
            if !updates.is_empty() {
                return Ok(updates);
            }
        }
    }
}

impl<'session> AccountBalancesByIdRequest<'session> {
    pub async fn get(self) -> Result<Vec<Asset>, SwaplockApiError> {
        get_account_balances_by_id(self.session, &self.account_id, self.asset_ids).await
    }

    pub async fn subscribe(
        self,
    ) -> Result<AccountBalancesSubscription<'session>, SwaplockApiError> {
        subscribe_account_balances_by_id(self.session, self.account_id, self.asset_ids).await
    }
}

impl<'session> AccountByIdRequest<'session> {
    pub async fn get(self) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_id(self.session, &self.account_id).await
    }

    pub async fn subscribe(self) -> Result<AccountSubscription<'session>, SwaplockApiError> {
        subscribe_account_by_id(self.session, self.account_id).await
    }
}

impl<'session> AccountByNameRequest<'session> {
    pub async fn get(self) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_name(self.session, &self.account_name).await
    }

    pub async fn subscribe(self) -> Result<AccountSubscription<'session>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        subscribe_account_by_id(self.session, account.id.0).await
    }
}

impl AccountSubscription<'_> {
    pub fn initial(&self) -> &AccountObject {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<AccountObject, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice()?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != ACCOUNT_CALLBACK_ID {
                continue;
            }

            return account_from_value("notice", payload, &self.account_id);
        }
    }
}

impl<'session> AssetByIdRequest<'session> {
    pub async fn get(self) -> Result<AssetObject, SwaplockApiError> {
        get_asset_by_id(self.session, &self.asset_id).await
    }

    pub async fn subscribe(self) -> Result<AssetSubscription<'session>, SwaplockApiError> {
        subscribe_asset_by_id(self.session, self.asset_id).await
    }
}

impl<'session> AssetBySymbolRequest<'session> {
    pub async fn get(self) -> Result<AssetObject, SwaplockApiError> {
        get_asset_by_symbol(self.session, &self.asset_symbol).await
    }

    pub async fn subscribe(self) -> Result<AssetSubscription<'session>, SwaplockApiError> {
        let asset = get_asset_by_symbol(self.session, &self.asset_symbol).await?;
        subscribe_asset_by_id(self.session, asset.id.0).await
    }
}

impl AssetSubscription<'_> {
    pub fn initial(&self) -> &AssetObject {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<AssetObject, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice()?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != ASSET_CALLBACK_ID {
                continue;
            }

            return asset_from_value("notice", payload, &self.asset_id);
        }
    }
}

impl AccountsRequest<'_> {
    pub async fn get(self) -> Result<Vec<Option<AccountObject>>, SwaplockApiError> {
        let value = self
            .session
            .database_call("get_accounts", json!([self.names_or_ids, false]))?;

        let accounts = value
            .as_array()
            .ok_or_else(|| SwaplockApiError::UnexpectedResponse {
                method: "get_accounts",
                message: "expected array response".to_string(),
            })?;

        accounts
            .iter()
            .map(|account| {
                if account.is_null() {
                    Ok(None)
                } else {
                    serde_json::from_value(account.clone())
                        .map(Some)
                        .map_err(|error| SwaplockApiError::UnexpectedResponse {
                            method: "get_accounts",
                            message: error.to_string(),
                        })
                }
            })
            .collect()
    }
}

impl ChainIdRequest<'_> {
    pub async fn get(self) -> Result<String, SwaplockApiError> {
        get_chain_id(self.session).await
    }
}

impl ChainPropertiesRequest<'_> {
    pub async fn get(self) -> Result<ChainPropertyObject, SwaplockApiError> {
        get_chain_properties(self.session).await
    }
}

impl<'session> DynamicGlobalPropertiesRequest<'session> {
    pub async fn get(self) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        get_dynamic_global_properties(self.session).await
    }

    pub async fn subscribe(
        self,
    ) -> Result<DynamicGlobalPropertiesSubscription<'session>, SwaplockApiError> {
        self.session.database_call(
            "set_subscribe_callback",
            json!([DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID, false]),
        )?;
        let value = self
            .session
            .database_call("get_objects", json!([[DYNAMIC_GLOBAL_PROPERTIES_ID], true]))?;
        let initial = dynamic_global_properties_from_value("get_objects", value)?;

        Ok(DynamicGlobalPropertiesSubscription {
            session: self.session,
            initial,
        })
    }
}

impl DynamicGlobalPropertiesSubscription<'_> {
    pub fn initial(&self) -> &DynamicGlobalPropertyObject {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice()?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID {
                continue;
            }

            return dynamic_global_properties_from_value("notice", payload);
        }
    }
}

impl GlobalPropertiesRequest<'_> {
    pub async fn get(self) -> Result<GlobalPropertyObject, SwaplockApiError> {
        get_global_properties(self.session).await
    }
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

async fn get_account_orders_by_id(
    session: &mut GrapheneSession,
    account_id: &str,
) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
    let value = session.database_call("get_full_accounts", json!([[account_id], false]))?;
    account_orders_from_full_accounts_value("get_full_accounts", value, account_id)
}

async fn subscribe_account_orders_by_id(
    session: &mut GrapheneSession,
    account_id: String,
) -> Result<AccountOrdersSubscription<'_>, SwaplockApiError> {
    session.database_call(
        "set_subscribe_callback",
        json!([ACCOUNT_ORDERS_CALLBACK_ID, false]),
    )?;
    let value = session.database_call("get_full_accounts", json!([[account_id], true]))?;
    let initial = account_orders_from_full_accounts_value("get_full_accounts", value, &account_id)?;

    Ok(AccountOrdersSubscription {
        session,
        account_id,
        initial,
    })
}

fn account_orders_from_full_accounts_value(
    method: &'static str,
    value: Value,
    account_id: &str,
) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
    let entries = value
        .as_array()
        .ok_or_else(|| unexpected_response(method, "expected full accounts array"))?;
    let account_entry = entries
        .iter()
        .find(|entry| {
            entry
                .as_array()
                .and_then(|row| row.first())
                .and_then(Value::as_str)
                == Some(account_id)
        })
        .ok_or_else(|| {
            unexpected_response(method, format!("missing `{account_id}` full account"))
        })?;
    let full_account = account_entry
        .as_array()
        .and_then(|row| row.get(1))
        .ok_or_else(|| unexpected_response(method, "missing full account payload"))?;
    let orders = full_account
        .get("limit_orders")
        .and_then(Value::as_array)
        .ok_or_else(|| unexpected_response(method, "missing limit_orders array"))?;

    let mut objects = Vec::new();
    for order in orders {
        let Some(seller) = order.get("seller").and_then(Value::as_str) else {
            return Err(unexpected_response(method, "limit order missing seller"));
        };
        if seller != account_id {
            continue;
        }
        let order: LimitOrderObject = serde_json::from_value(order.clone()).map_err(|error| {
            SwaplockApiError::UnexpectedResponse {
                method,
                message: error.to_string(),
            }
        })?;
        objects.push(order);
    }

    Ok(objects)
}

fn collect_account_order_objects(
    method: &'static str,
    value: &Value,
    account_id: &str,
) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
    let mut objects = Vec::new();
    collect_account_order_objects_inner(method, value, account_id, &mut objects)?;
    Ok(objects)
}

fn collect_account_order_objects_inner(
    method: &'static str,
    value: &Value,
    account_id: &str,
    objects: &mut Vec<LimitOrderObject>,
) -> Result<(), SwaplockApiError> {
    if let Some(object) = value.as_object() {
        if object.get("seller").and_then(Value::as_str) == Some(account_id) {
            let order: LimitOrderObject =
                serde_json::from_value(value.clone()).map_err(|error| {
                    SwaplockApiError::UnexpectedResponse {
                        method,
                        message: error.to_string(),
                    }
                })?;
            objects.push(order);
        }
    }

    if let Some(array) = value.as_array() {
        for item in array {
            collect_account_order_objects_inner(method, item, account_id, objects)?;
        }
    }

    Ok(())
}

async fn get_account_balances_by_id(
    session: &mut GrapheneSession,
    account_id: &str,
    asset_ids: Vec<String>,
) -> Result<Vec<Asset>, SwaplockApiError> {
    let value = session.database_call("get_account_balances", json!([account_id, asset_ids]))?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_account_balances",
        message: error.to_string(),
    })
}

async fn get_account_by_name(
    session: &mut GrapheneSession,
    account_name: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let accounts = session.database_call("lookup_accounts", json!([account_name, 1]))?;
    let account_id = accounts
        .as_array()
        .and_then(|rows| {
            rows.iter().find_map(|row| {
                let row = row.as_array()?;
                let name = row.first()?.as_str()?;
                let id = row.get(1)?.as_str()?;
                (name == account_name).then(|| id.to_string())
            })
        })
        .ok_or_else(|| SwaplockApiError::AccountNotFound {
            account: account_name.to_string(),
        })?;

    get_account_by_id(session, &account_id).await
}

async fn get_account_by_id(
    session: &mut GrapheneSession,
    account_id: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let objects = session.database_call("get_objects", json!([[account_id]]))?;
    account_from_value("get_objects", objects, account_id)
}

async fn get_asset_by_symbol(
    session: &mut GrapheneSession,
    asset_symbol: &str,
) -> Result<AssetObject, SwaplockApiError> {
    let value = session.database_call("lookup_asset_symbols", json!([[asset_symbol]]))?;
    let asset = value
        .as_array()
        .and_then(|assets| assets.first())
        .filter(|asset| !asset.is_null())
        .cloned()
        .ok_or_else(|| SwaplockApiError::AssetNotFound {
            asset: asset_symbol.to_string(),
        })?;

    let asset: AssetObject =
        serde_json::from_value(asset).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "lookup_asset_symbols",
            message: error.to_string(),
        })?;

    if asset.symbol != asset_symbol {
        return Err(SwaplockApiError::AssetNotFound {
            asset: asset_symbol.to_string(),
        });
    }

    Ok(asset)
}

async fn get_asset_by_id(
    session: &mut GrapheneSession,
    asset_id: &str,
) -> Result<AssetObject, SwaplockApiError> {
    let objects = session.database_call("get_objects", json!([[asset_id]]))?;
    asset_from_value("get_objects", objects, asset_id)
}

async fn get_chain_id(session: &mut GrapheneSession) -> Result<String, SwaplockApiError> {
    let value = session.database_call("get_chain_id", json!([]))?;
    Ok(parse_chain_id(value)?)
}

async fn get_chain_properties(
    session: &mut GrapheneSession,
) -> Result<ChainPropertyObject, SwaplockApiError> {
    let value = session.database_call("get_chain_properties", json!([]))?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_chain_properties",
        message: error.to_string(),
    })
}

async fn get_dynamic_global_properties(
    session: &mut GrapheneSession,
) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
    let value = session.database_call("get_dynamic_global_properties", json!([]))?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_dynamic_global_properties",
        message: error.to_string(),
    })
}

async fn get_global_properties(
    session: &mut GrapheneSession,
) -> Result<GlobalPropertyObject, SwaplockApiError> {
    let value = session.database_call("get_global_properties", json!([]))?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_global_properties",
        message: error.to_string(),
    })
}

async fn subscribe_account_balances_by_id(
    session: &mut GrapheneSession,
    account_id: String,
    asset_ids: Vec<String>,
) -> Result<AccountBalancesSubscription<'_>, SwaplockApiError> {
    session.database_call(
        "set_subscribe_callback",
        json!([ACCOUNT_BALANCES_CALLBACK_ID, false]),
    )?;
    let value = session.database_call("get_full_accounts", json!([[account_id], true]))?;
    let initial = account_balances_from_full_accounts_value(
        "get_full_accounts",
        value,
        &account_id,
        &asset_ids,
    )?;

    Ok(AccountBalancesSubscription {
        session,
        account_id,
        asset_ids,
        initial,
    })
}

fn account_balances_from_full_accounts_value(
    method: &'static str,
    value: Value,
    account_id: &str,
    asset_ids: &[String],
) -> Result<Vec<AccountBalanceObject>, SwaplockApiError> {
    let entries = value
        .as_array()
        .ok_or_else(|| unexpected_response(method, "expected full accounts array"))?;
    let account_entry = entries
        .iter()
        .find(|entry| {
            entry
                .as_array()
                .and_then(|row| row.first())
                .and_then(Value::as_str)
                == Some(account_id)
        })
        .ok_or_else(|| {
            unexpected_response(method, format!("missing `{account_id}` full account"))
        })?;
    let full_account = account_entry
        .as_array()
        .and_then(|row| row.get(1))
        .ok_or_else(|| unexpected_response(method, "missing full account payload"))?;
    let balances = full_account
        .get("balances")
        .and_then(Value::as_array)
        .ok_or_else(|| unexpected_response(method, "missing balances array"))?;

    let mut objects = Vec::new();
    for balance in balances {
        let Some(asset_type) = balance.get("asset_type").and_then(Value::as_str) else {
            return Err(unexpected_response(method, "balance missing asset_type"));
        };
        if !asset_ids.is_empty() && !asset_ids.iter().any(|asset_id| asset_id == asset_type) {
            continue;
        }
        let balance: AccountBalanceObject =
            serde_json::from_value(balance.clone()).map_err(|error| {
                SwaplockApiError::UnexpectedResponse {
                    method,
                    message: error.to_string(),
                }
            })?;
        objects.push(balance);
    }

    Ok(objects)
}

fn collect_account_balance_objects(
    method: &'static str,
    value: &Value,
    account_id: &str,
    asset_ids: &[String],
) -> Result<Vec<AccountBalanceObject>, SwaplockApiError> {
    let mut objects = Vec::new();
    collect_account_balance_objects_inner(method, value, account_id, asset_ids, &mut objects)?;
    Ok(objects)
}

fn collect_account_balance_objects_inner(
    method: &'static str,
    value: &Value,
    account_id: &str,
    asset_ids: &[String],
    objects: &mut Vec<AccountBalanceObject>,
) -> Result<(), SwaplockApiError> {
    if let Some(object) = value.as_object() {
        let owner = object.get("owner").and_then(Value::as_str);
        let asset_type = object.get("asset_type").and_then(Value::as_str);
        if owner == Some(account_id)
            && (asset_ids.is_empty()
                || asset_type.is_some_and(|asset_type| asset_ids.iter().any(|id| id == asset_type)))
        {
            let balance: AccountBalanceObject =
                serde_json::from_value(value.clone()).map_err(|error| {
                    SwaplockApiError::UnexpectedResponse {
                        method,
                        message: error.to_string(),
                    }
                })?;
            objects.push(balance);
        }
    }

    if let Some(array) = value.as_array() {
        for item in array {
            collect_account_balance_objects_inner(method, item, account_id, asset_ids, objects)?;
        }
    }

    Ok(())
}

fn unexpected_response(method: &'static str, message: impl Into<String>) -> SwaplockApiError {
    SwaplockApiError::UnexpectedResponse {
        method,
        message: message.into(),
    }
}

async fn subscribe_account_by_id(
    session: &mut GrapheneSession,
    account_id: String,
) -> Result<AccountSubscription<'_>, SwaplockApiError> {
    session.database_call(
        "set_subscribe_callback",
        json!([ACCOUNT_CALLBACK_ID, false]),
    )?;
    let value = session.database_call("get_objects", json!([[account_id], true]))?;
    let initial = account_from_value("get_objects", value, &account_id)?;

    Ok(AccountSubscription {
        session,
        account_id,
        initial,
    })
}

async fn subscribe_asset_by_id(
    session: &mut GrapheneSession,
    asset_id: String,
) -> Result<AssetSubscription<'_>, SwaplockApiError> {
    session.database_call("set_subscribe_callback", json!([ASSET_CALLBACK_ID, false]))?;
    let value = session.database_call("get_objects", json!([[asset_id], true]))?;
    let initial = asset_from_value("get_objects", value, &asset_id)?;

    Ok(AssetSubscription {
        session,
        asset_id,
        initial,
    })
}

fn asset_from_value(
    method: &'static str,
    value: Value,
    asset_id: &str,
) -> Result<AssetObject, SwaplockApiError> {
    let value = find_object_by_id(&value, asset_id).ok_or_else(|| {
        SwaplockApiError::UnexpectedResponse {
            method,
            message: format!("missing `{asset_id}` asset object"),
        }
    })?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}

fn account_from_value(
    method: &'static str,
    value: Value,
    account_id: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let value = find_object_by_id(&value, account_id).ok_or_else(|| {
        SwaplockApiError::UnexpectedResponse {
            method,
            message: format!("missing `{account_id}` account object"),
        }
    })?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}

fn dynamic_global_properties_from_value(
    method: &'static str,
    value: Value,
) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
    let value = find_object_by_id(&value, DYNAMIC_GLOBAL_PROPERTIES_ID).ok_or_else(|| {
        SwaplockApiError::UnexpectedResponse {
            method,
            message: format!("missing `{DYNAMIC_GLOBAL_PROPERTIES_ID}` object"),
        }
    })?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}

fn find_object_by_id(value: &Value, id: &str) -> Option<Value> {
    if value.get("id").and_then(Value::as_str) == Some(id) {
        return Some(value.clone());
    }

    value
        .as_array()?
        .iter()
        .find_map(|item| find_object_by_id(item, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_account_balance_objects_from_full_accounts_response() {
        let asset_ids = vec!["1.3.0".to_string()];
        let balances = account_balances_from_full_accounts_value(
            "get_full_accounts",
            json!([["1.2.100", {
                "balances": [
                    {
                        "id": "2.5.1",
                        "owner": "1.2.100",
                        "asset_type": "1.3.0",
                        "balance": "42",
                        "maintenance_flag": false
                    },
                    {
                        "id": "2.5.2",
                        "owner": "1.2.100",
                        "asset_type": "1.3.1",
                        "balance": 7,
                        "maintenance_flag": false
                    }
                ]
            }]]),
            "1.2.100",
            &asset_ids,
        )
        .unwrap();

        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].id.0, "2.5.1");
        assert_eq!(balances[0].owner.0, "1.2.100");
        assert_eq!(balances[0].asset_type.0, "1.3.0");
        assert_eq!(balances[0].balance, 42);
    }

    #[test]
    fn empty_asset_filter_parses_all_account_balance_objects_from_full_accounts_response() {
        let asset_ids = Vec::new();
        let balances = account_balances_from_full_accounts_value(
            "get_full_accounts",
            json!([["1.2.100", {
                "balances": [
                    {
                        "id": "2.5.1",
                        "owner": "1.2.100",
                        "asset_type": "1.3.0",
                        "balance": "42",
                        "maintenance_flag": false
                    },
                    {
                        "id": "2.5.2",
                        "owner": "1.2.100",
                        "asset_type": "1.3.1",
                        "balance": 7,
                        "maintenance_flag": false
                    }
                ]
            }]]),
            "1.2.100",
            &asset_ids,
        )
        .unwrap();

        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0].id.0, "2.5.1");
        assert_eq!(balances[1].id.0, "2.5.2");
    }

    #[test]
    fn collects_only_matching_account_balance_notice_objects() {
        let asset_ids = vec!["1.3.0".to_string()];
        let balances = collect_account_balance_objects(
            "notice",
            &json!([[
                {
                    "id": "2.5.1",
                    "owner": "1.2.100",
                    "asset_type": "1.3.0",
                    "balance": "42",
                    "maintenance_flag": false
                },
                {
                    "id": "2.5.2",
                    "owner": "1.2.100",
                    "asset_type": "1.3.1",
                    "balance": 7,
                    "maintenance_flag": false
                },
                {
                    "id": "1.2.100",
                    "name": "swaplock"
                }
            ]]),
            "1.2.100",
            &asset_ids,
        )
        .unwrap();

        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].id.0, "2.5.1");
        assert_eq!(balances[0].balance, 42);
    }

    #[test]
    fn empty_asset_filter_collects_all_account_balance_notice_objects_for_owner() {
        let asset_ids = Vec::new();
        let balances = collect_account_balance_objects(
            "notice",
            &json!([[
                {
                    "id": "2.5.1",
                    "owner": "1.2.100",
                    "asset_type": "1.3.0",
                    "balance": "42",
                    "maintenance_flag": false
                },
                {
                    "id": "2.5.2",
                    "owner": "1.2.100",
                    "asset_type": "1.3.1",
                    "balance": 7,
                    "maintenance_flag": false
                },
                {
                    "id": "2.5.3",
                    "owner": "1.2.101",
                    "asset_type": "1.3.0",
                    "balance": 9,
                    "maintenance_flag": false
                }
            ]]),
            "1.2.100",
            &asset_ids,
        )
        .unwrap();

        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0].id.0, "2.5.1");
        assert_eq!(balances[1].id.0, "2.5.2");
    }

    fn limit_order_fixture(id: &str, seller: &str) -> Value {
        json!({
            "id": id,
            "expiration": "2026-01-01T00:00:00",
            "seller": seller,
            "for_sale": "1000",
            "sell_price": {
                "base": {"amount": 1000, "asset_id": "1.3.0"},
                "quote": {"amount": 2000, "asset_id": "1.3.1"}
            },
            "filled_amount": "0",
            "deferred_fee": 0,
            "deferred_paid_fee": {"amount": 0, "asset_id": "1.3.0"},
            "is_settled_debt": false,
            "on_fill": [],
            "take_profit_order_id": null
        })
    }

    #[test]
    fn parses_account_orders_from_full_accounts_response() {
        let orders = account_orders_from_full_accounts_value(
            "get_full_accounts",
            json!([["1.2.100", {
                "limit_orders": [
                    limit_order_fixture("1.7.10", "1.2.100"),
                    limit_order_fixture("1.7.11", "1.2.101")
                ]
            }]]),
            "1.2.100",
        )
        .unwrap();

        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].id.0, "1.7.10");
        assert_eq!(orders[0].seller.0, "1.2.100");
        assert_eq!(orders[0].for_sale, 1000);
    }

    #[test]
    fn collects_only_matching_account_order_notice_objects() {
        let orders = collect_account_order_objects(
            "notice",
            &json!([[
                limit_order_fixture("1.7.10", "1.2.100"),
                limit_order_fixture("1.7.11", "1.2.101"),
                {"id": "1.2.100", "name": "swaplock"}
            ]]),
            "1.2.100",
        )
        .unwrap();

        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].id.0, "1.7.10");
    }
}
