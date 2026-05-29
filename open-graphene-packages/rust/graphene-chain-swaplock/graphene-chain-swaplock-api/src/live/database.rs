use std::time::{Duration, Instant};

use graphene_chain_swaplock_bindings::generated::{
    AccountBalanceObject, AccountObject, AssetObject, DynamicGlobalPropertyObject, LimitOrderObject,
};
use open_graphene_transport::{CallbackId, LiveSubscription, LiveTransportHandle};
use serde_json::json;

use crate::SwaplockApiError;
use crate::database::{
    DYNAMIC_GLOBAL_PROPERTIES_ID, account_balances_from_full_accounts_value, account_from_value,
    account_orders_from_full_accounts_value, asset_from_value, collect_account_balance_objects,
    collect_account_order_objects, dynamic_global_properties_from_value,
};

use super::{LIVE_DATABASE_CALLBACK_ID, remaining_or_callback_timeout};

pub struct SwaplockLiveDatabaseApi {
    pub(crate) live: LiveTransportHandle,
    pub(crate) database_api_id: u64,
}

pub struct SwaplockLiveDynamicGlobalPropertiesSubscription {
    initial: DynamicGlobalPropertyObject,
    subscription: LiveSubscription,
}

pub struct SwaplockLiveAccountSubscription {
    account_id: String,
    initial: AccountObject,
    subscription: LiveSubscription,
}

pub struct SwaplockLiveAssetSubscription {
    asset_id: String,
    initial: AssetObject,
    subscription: LiveSubscription,
}

pub struct SwaplockLiveAccountBalancesSubscription {
    account_id: String,
    asset_ids: Vec<String>,
    initial: Vec<AccountBalanceObject>,
    subscription: LiveSubscription,
}

pub struct SwaplockLiveAccountOrdersSubscription {
    account_id: String,
    initial: Vec<LimitOrderObject>,
    subscription: LiveSubscription,
}

impl SwaplockLiveDatabaseApi {
    pub fn subscribe_dynamic_global_properties(
        self,
    ) -> Result<SwaplockLiveDynamicGlobalPropertiesSubscription, SwaplockApiError> {
        self.subscribe_dynamic_global_properties_timeout(Duration::from_secs(10))
    }

    pub fn subscribe_dynamic_global_properties_timeout(
        self,
        timeout: Duration,
    ) -> Result<SwaplockLiveDynamicGlobalPropertiesSubscription, SwaplockApiError> {
        let callback_id = CallbackId::new(LIVE_DATABASE_CALLBACK_ID);
        let subscription = self.live.subscribe_callback(callback_id)?;
        self.live
            .call(
                self.database_api_id,
                "set_subscribe_callback",
                json!([LIVE_DATABASE_CALLBACK_ID, false]),
            )?
            .wait_timeout(timeout)?;

        let value = self
            .live
            .call(
                self.database_api_id,
                "get_objects",
                json!([[DYNAMIC_GLOBAL_PROPERTIES_ID], true]),
            )?
            .wait_timeout(timeout)?;
        let initial = dynamic_global_properties_from_value("get_objects", value)?;

        Ok(SwaplockLiveDynamicGlobalPropertiesSubscription {
            initial,
            subscription,
        })
    }

    pub fn account_by_id(self, account_id: impl Into<String>) -> SwaplockLiveAccountByIdRequest {
        SwaplockLiveAccountByIdRequest {
            live: self.live,
            database_api_id: self.database_api_id,
            account_id: account_id.into(),
        }
    }

    pub fn asset_by_id(self, asset_id: impl Into<String>) -> SwaplockLiveAssetByIdRequest {
        SwaplockLiveAssetByIdRequest {
            live: self.live,
            database_api_id: self.database_api_id,
            asset_id: asset_id.into(),
        }
    }

    pub fn account_balances_by_id(
        self,
        account_id: impl Into<String>,
        asset_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> SwaplockLiveAccountBalancesByIdRequest {
        SwaplockLiveAccountBalancesByIdRequest {
            live: self.live,
            database_api_id: self.database_api_id,
            account_id: account_id.into(),
            asset_ids: asset_ids.into_iter().map(Into::into).collect(),
        }
    }

    pub fn account_orders_by_id(
        self,
        account_id: impl Into<String>,
    ) -> SwaplockLiveAccountOrdersByIdRequest {
        SwaplockLiveAccountOrdersByIdRequest {
            live: self.live,
            database_api_id: self.database_api_id,
            account_id: account_id.into(),
        }
    }
}

pub struct SwaplockLiveAccountByIdRequest {
    live: LiveTransportHandle,
    database_api_id: u64,
    account_id: String,
}

pub struct SwaplockLiveAssetByIdRequest {
    live: LiveTransportHandle,
    database_api_id: u64,
    asset_id: String,
}

pub struct SwaplockLiveAccountBalancesByIdRequest {
    live: LiveTransportHandle,
    database_api_id: u64,
    account_id: String,
    asset_ids: Vec<String>,
}

pub struct SwaplockLiveAccountOrdersByIdRequest {
    live: LiveTransportHandle,
    database_api_id: u64,
    account_id: String,
}

impl SwaplockLiveAccountByIdRequest {
    pub fn subscribe(self) -> Result<SwaplockLiveAccountSubscription, SwaplockApiError> {
        self.subscribe_timeout(Duration::from_secs(10))
    }

    pub fn subscribe_timeout(
        self,
        timeout: Duration,
    ) -> Result<SwaplockLiveAccountSubscription, SwaplockApiError> {
        let callback_id = CallbackId::new(LIVE_DATABASE_CALLBACK_ID);
        let subscription = self.live.subscribe_callback(callback_id)?;
        self.live
            .call(
                self.database_api_id,
                "set_subscribe_callback",
                json!([LIVE_DATABASE_CALLBACK_ID, false]),
            )?
            .wait_timeout(timeout)?;

        let value = self
            .live
            .call(
                self.database_api_id,
                "get_objects",
                json!([[self.account_id.clone()], true]),
            )?
            .wait_timeout(timeout)?;
        let initial = account_from_value("get_objects", value, &self.account_id)?;

        Ok(SwaplockLiveAccountSubscription {
            account_id: self.account_id,
            initial,
            subscription,
        })
    }
}

impl SwaplockLiveAssetByIdRequest {
    pub fn subscribe(self) -> Result<SwaplockLiveAssetSubscription, SwaplockApiError> {
        self.subscribe_timeout(Duration::from_secs(10))
    }

    pub fn subscribe_timeout(
        self,
        timeout: Duration,
    ) -> Result<SwaplockLiveAssetSubscription, SwaplockApiError> {
        let callback_id = CallbackId::new(LIVE_DATABASE_CALLBACK_ID);
        let subscription = self.live.subscribe_callback(callback_id)?;
        self.live
            .call(
                self.database_api_id,
                "set_subscribe_callback",
                json!([LIVE_DATABASE_CALLBACK_ID, false]),
            )?
            .wait_timeout(timeout)?;

        let value = self
            .live
            .call(
                self.database_api_id,
                "get_objects",
                json!([[self.asset_id.clone()], true]),
            )?
            .wait_timeout(timeout)?;
        let initial = asset_from_value("get_objects", value, &self.asset_id)?;

        Ok(SwaplockLiveAssetSubscription {
            asset_id: self.asset_id,
            initial,
            subscription,
        })
    }
}

impl SwaplockLiveAccountBalancesByIdRequest {
    pub fn subscribe(self) -> Result<SwaplockLiveAccountBalancesSubscription, SwaplockApiError> {
        self.subscribe_timeout(Duration::from_secs(10))
    }

    pub fn subscribe_timeout(
        self,
        timeout: Duration,
    ) -> Result<SwaplockLiveAccountBalancesSubscription, SwaplockApiError> {
        let callback_id = CallbackId::new(LIVE_DATABASE_CALLBACK_ID);
        let subscription = self.live.subscribe_callback(callback_id)?;
        self.live
            .call(
                self.database_api_id,
                "set_subscribe_callback",
                json!([LIVE_DATABASE_CALLBACK_ID, false]),
            )?
            .wait_timeout(timeout)?;

        let value = self
            .live
            .call(
                self.database_api_id,
                "get_full_accounts",
                json!([[self.account_id.clone()], true]),
            )?
            .wait_timeout(timeout)?;
        let initial = account_balances_from_full_accounts_value(
            "get_full_accounts",
            value,
            &self.account_id,
            &self.asset_ids,
        )?;

        Ok(SwaplockLiveAccountBalancesSubscription {
            account_id: self.account_id,
            asset_ids: self.asset_ids,
            initial,
            subscription,
        })
    }
}

impl SwaplockLiveAccountOrdersByIdRequest {
    pub fn subscribe(self) -> Result<SwaplockLiveAccountOrdersSubscription, SwaplockApiError> {
        self.subscribe_timeout(Duration::from_secs(10))
    }

    pub fn subscribe_timeout(
        self,
        timeout: Duration,
    ) -> Result<SwaplockLiveAccountOrdersSubscription, SwaplockApiError> {
        let callback_id = CallbackId::new(LIVE_DATABASE_CALLBACK_ID);
        let subscription = self.live.subscribe_callback(callback_id)?;
        self.live
            .call(
                self.database_api_id,
                "set_subscribe_callback",
                json!([LIVE_DATABASE_CALLBACK_ID, false]),
            )?
            .wait_timeout(timeout)?;

        let value = self
            .live
            .call(
                self.database_api_id,
                "get_full_accounts",
                json!([[self.account_id.clone()], true]),
            )?
            .wait_timeout(timeout)?;
        let initial =
            account_orders_from_full_accounts_value("get_full_accounts", value, &self.account_id)?;

        Ok(SwaplockLiveAccountOrdersSubscription {
            account_id: self.account_id,
            initial,
            subscription,
        })
    }
}

impl SwaplockLiveDynamicGlobalPropertiesSubscription {
    pub fn initial(&self) -> &DynamicGlobalPropertyObject {
        &self.initial
    }

    pub fn next_update(&self) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        loop {
            let value = self.subscription.next()?;
            if let Ok(properties) = dynamic_global_properties_from_value("notice", value) {
                return Ok(properties);
            }
        }
    }

    pub fn next_update_timeout(
        &self,
        timeout: Duration,
    ) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining =
                remaining_or_callback_timeout(deadline, self.subscription.callback_id(), timeout)?;
            let value = self.subscription.next_timeout(remaining)?;
            if let Ok(properties) = parse_live_dynamic_global_properties_update(value) {
                return Ok(properties);
            }
        }
    }
}

impl SwaplockLiveAccountSubscription {
    pub fn initial(&self) -> &AccountObject {
        &self.initial
    }

    pub fn next_update(&self) -> Result<AccountObject, SwaplockApiError> {
        loop {
            let value = self.subscription.next()?;
            if let Ok(account) = parse_live_account_update(value, &self.account_id) {
                return Ok(account);
            }
        }
    }

    pub fn next_update_timeout(
        &self,
        timeout: Duration,
    ) -> Result<AccountObject, SwaplockApiError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining =
                remaining_or_callback_timeout(deadline, self.subscription.callback_id(), timeout)?;
            let value = self.subscription.next_timeout(remaining)?;
            if let Ok(account) = parse_live_account_update(value, &self.account_id) {
                return Ok(account);
            }
        }
    }
}

impl SwaplockLiveAssetSubscription {
    pub fn initial(&self) -> &AssetObject {
        &self.initial
    }

    pub fn next_update(&self) -> Result<AssetObject, SwaplockApiError> {
        loop {
            let value = self.subscription.next()?;
            if let Ok(asset) = parse_live_asset_update(value, &self.asset_id) {
                return Ok(asset);
            }
        }
    }

    pub fn next_update_timeout(&self, timeout: Duration) -> Result<AssetObject, SwaplockApiError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining =
                remaining_or_callback_timeout(deadline, self.subscription.callback_id(), timeout)?;
            let value = self.subscription.next_timeout(remaining)?;
            if let Ok(asset) = parse_live_asset_update(value, &self.asset_id) {
                return Ok(asset);
            }
        }
    }
}

impl SwaplockLiveAccountBalancesSubscription {
    pub fn initial(&self) -> &[AccountBalanceObject] {
        &self.initial
    }

    pub fn next_update(&self) -> Result<Vec<AccountBalanceObject>, SwaplockApiError> {
        loop {
            let value = self.subscription.next()?;
            let updates =
                parse_live_account_balance_updates(value, &self.account_id, &self.asset_ids)?;
            if !updates.is_empty() {
                return Ok(updates);
            }
        }
    }

    pub fn next_update_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Vec<AccountBalanceObject>, SwaplockApiError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining =
                remaining_or_callback_timeout(deadline, self.subscription.callback_id(), timeout)?;
            let value = self.subscription.next_timeout(remaining)?;
            let updates =
                parse_live_account_balance_updates(value, &self.account_id, &self.asset_ids)?;
            if !updates.is_empty() {
                return Ok(updates);
            }
        }
    }
}

impl SwaplockLiveAccountOrdersSubscription {
    pub fn initial(&self) -> &[LimitOrderObject] {
        &self.initial
    }

    pub fn next_update(&self) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        loop {
            let value = self.subscription.next()?;
            let updates = parse_live_account_order_updates(value, &self.account_id)?;
            if !updates.is_empty() {
                return Ok(updates);
            }
        }
    }

    pub fn next_update_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining =
                remaining_or_callback_timeout(deadline, self.subscription.callback_id(), timeout)?;
            let value = self.subscription.next_timeout(remaining)?;
            let updates = parse_live_account_order_updates(value, &self.account_id)?;
            if !updates.is_empty() {
                return Ok(updates);
            }
        }
    }
}

fn parse_live_dynamic_global_properties_update(
    value: serde_json::Value,
) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
    dynamic_global_properties_from_value("notice", value)
}

fn parse_live_account_update(
    value: serde_json::Value,
    account_id: &str,
) -> Result<AccountObject, SwaplockApiError> {
    account_from_value("notice", value, account_id)
}

fn parse_live_asset_update(
    value: serde_json::Value,
    asset_id: &str,
) -> Result<AssetObject, SwaplockApiError> {
    asset_from_value("notice", value, asset_id)
}

fn parse_live_account_balance_updates(
    value: serde_json::Value,
    account_id: &str,
    asset_ids: &[String],
) -> Result<Vec<AccountBalanceObject>, SwaplockApiError> {
    collect_account_balance_objects("notice", &value, account_id, asset_ids)
}

fn parse_live_account_order_updates(
    value: serde_json::Value,
    account_id: &str,
) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
    collect_account_order_objects("notice", &value, account_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn live_dynamic_global_properties_parser_accepts_notice_payload() {
        let properties = parse_live_dynamic_global_properties_update(json!([[{
            "id": "2.1.0",
            "head_block_number": 123,
            "head_block_id": "0000007b00000000000000000000000000000000",
            "time": "2026-01-01T00:00:00",
            "current_witness": "1.6.1",
            "next_maintenance_time": "2026-01-01T01:00:00",
            "last_budget_time": "2026-01-01T00:00:00",
            "witness_budget": 0,
            "accounts_registered_this_interval": 0,
            "recently_missed_count": 0,
            "current_aslot": 456,
            "recent_slots_filled": "340282366920938463463374607431768211455",
            "dynamic_flags": 0,
            "last_irreversible_block_num": 120,
            "last_vote_tally_time": "2026-01-01T00:00:00",
            "total_pob": 0,
            "total_inactive": 0
        }]]))
        .unwrap();

        assert_eq!(properties.id.0, "2.1.0");
        assert_eq!(properties.head_block_number, 123);
        assert_eq!(properties.current_aslot, 456);
    }

    #[test]
    fn live_dynamic_global_properties_parser_rejects_irrelevant_notice_payload() {
        let error = parse_live_dynamic_global_properties_update(json!([[{
            "id": "1.2.100"
        }]]))
        .unwrap_err();

        assert!(
            matches!(error, SwaplockApiError::UnexpectedResponse { method, .. } if method == "notice")
        );
    }

    #[test]
    fn live_account_parser_accepts_matching_notice_payload() {
        let account = parse_live_account_update(
            json!([[{
                "id": "1.2.100",
                "membership_expiration_date": "1970-01-01T00:00:00",
                "registrar": "1.2.0",
                "referrer": "1.2.0",
                "lifetime_referrer": "1.2.0",
                "network_fee_percentage": 0,
                "lifetime_referrer_fee_percentage": 0,
                "referrer_rewards_percentage": 0,
                "name": "swaplock",
                "owner": {"weight_threshold": 1, "account_auths": [], "key_auths": [], "address_auths": []},
                "active": {"weight_threshold": 1, "account_auths": [], "key_auths": [], "address_auths": []},
                "options": {
                    "memo_key": "BTS1111111111111111111111111111111114T1Anm",
                    "voting_account": "1.2.5",
                    "num_witness": 0,
                    "num_committee": 0,
                    "votes": [],
                    "extensions": []
                },
                "num_committee_voted": 0,
                "statistics": "2.6.100",
                "whitelisting_accounts": [],
                "blacklisting_accounts": [],
                "whitelisted_accounts": [],
                "blacklisted_accounts": [],
                "owner_special_authority": [0, {}],
                "active_special_authority": [0, {}],
                "top_n_control_flags": 0,
                "allowed_assets": null,
                "creation_block_num": 1,
                "creation_time": "2026-01-01T00:00:00"
            }]]),
            "1.2.100",
        )
        .unwrap();

        assert_eq!(account.id.0, "1.2.100");
        assert_eq!(account.name, "swaplock");
    }

    #[test]
    fn live_account_parser_rejects_non_matching_notice_payload() {
        let error = parse_live_account_update(json!([[{"id": "1.2.101"}]]), "1.2.100").unwrap_err();
        assert!(
            matches!(error, SwaplockApiError::UnexpectedResponse { method, .. } if method == "notice")
        );
    }

    #[test]
    fn live_asset_parser_accepts_matching_notice_payload() {
        let asset = parse_live_asset_update(
            json!([[{
                "id": "1.3.0",
                "symbol": "BTS",
                "precision": 5,
                "issuer": "1.2.0",
                "options": {
                    "max_supply": "1000000000000000",
                    "market_fee_percent": 0,
                    "max_market_fee": 0,
                    "issuer_permissions": 0,
                    "flags": 0,
                    "core_exchange_rate": {
                        "base": {"amount": 1, "asset_id": "1.3.0"},
                        "quote": {"amount": 1, "asset_id": "1.3.0"}
                    },
                    "whitelist_authorities": [],
                    "blacklist_authorities": [],
                    "whitelist_markets": [],
                    "blacklist_markets": [],
                    "description": "",
                    "extensions": {
                        "reward_percent": null,
                        "whitelist_market_fee_sharing": null,
                        "taker_fee_percent": null
                    }
                },
                "dynamic_asset_data_id": "2.3.0",
                "bitasset_data_id": null,
                "buyback_account": null,
                "for_liquidity_pool": null,
                "creation_block_num": 1,
                "creation_time": "2026-01-01T00:00:00"
            }]]),
            "1.3.0",
        )
        .unwrap();

        assert_eq!(asset.id.0, "1.3.0");
        assert_eq!(asset.symbol, "BTS");
        assert_eq!(asset.precision, 5);
    }

    #[test]
    fn live_asset_parser_rejects_non_matching_notice_payload() {
        let error = parse_live_asset_update(json!([[{"id": "1.3.1"}]]), "1.3.0").unwrap_err();
        assert!(
            matches!(error, SwaplockApiError::UnexpectedResponse { method, .. } if method == "notice")
        );
    }

    #[test]
    fn live_account_balance_parser_collects_matching_notice_objects() {
        let asset_ids = vec!["1.3.0".to_string()];
        let balances = parse_live_account_balance_updates(
            json!([[
                {"id": "2.5.1", "owner": "1.2.100", "asset_type": "1.3.0", "balance": 42, "maintenance_flag": false},
                {"id": "2.5.2", "owner": "1.2.100", "asset_type": "1.3.1", "balance": 7, "maintenance_flag": false},
                {"id": "2.5.3", "owner": "1.2.101", "asset_type": "1.3.0", "balance": 9, "maintenance_flag": false}
            ]]),
            "1.2.100",
            &asset_ids,
        )
        .unwrap();

        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].id.0, "2.5.1");
        assert_eq!(balances[0].asset_type.0, "1.3.0");
        assert_eq!(balances[0].balance, 42);
    }

    #[test]
    fn live_account_balance_parser_allows_empty_asset_filter() {
        let balances = parse_live_account_balance_updates(
            json!([[
                {"id": "2.5.1", "owner": "1.2.100", "asset_type": "1.3.0", "balance": 42, "maintenance_flag": false},
                {"id": "2.5.2", "owner": "1.2.100", "asset_type": "1.3.1", "balance": 7, "maintenance_flag": false}
            ]]),
            "1.2.100",
            &[],
        )
        .unwrap();

        assert_eq!(balances.len(), 2);
    }

    fn limit_order_fixture(id: &str, seller: &str) -> serde_json::Value {
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
    fn live_account_order_parser_collects_matching_notice_objects() {
        let orders = parse_live_account_order_updates(
            json!([[
                limit_order_fixture("1.7.10", "1.2.100"),
                limit_order_fixture("1.7.11", "1.2.101"),
                {"id": "1.2.100", "name": "swaplock"}
            ]]),
            "1.2.100",
        )
        .unwrap();

        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].id.0, "1.7.10");
        assert_eq!(orders[0].seller.0, "1.2.100");
    }

    #[test]
    fn live_account_order_parser_returns_empty_for_irrelevant_notice() {
        let orders = parse_live_account_order_updates(
            json!([[limit_order_fixture("1.7.11", "1.2.101")]]),
            "1.2.100",
        )
        .unwrap();

        assert!(orders.is_empty());
    }
}
