use graphene_chain_swaplock_bindings::generated::{AccountBalanceObject, Asset};
use open_graphene_transport::{CallbackId, GrapheneSession, JsonRpcInbound};
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::constants::ACCOUNT_BALANCES_CALLBACK_ID;
use super::error::unexpected_response;
use super::string_list::IntoStringList;

pub struct AccountBalancesByIdRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_id: String,
    pub(super) asset_ids: Vec<String>,
}

pub struct AccountBalancesSubscription<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    asset_ids: Vec<String>,
    initial: Vec<AccountBalanceObject>,
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

impl AccountBalancesSubscription<'_> {
    pub fn initial(&self) -> &[AccountBalanceObject] {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<Vec<AccountBalanceObject>, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice().await?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != CallbackId::new(ACCOUNT_BALANCES_CALLBACK_ID) {
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

pub(super) async fn get_account_balances_by_id<L>(
    session: &mut GrapheneSession,
    account_id: &str,
    asset_ids: L,
) -> Result<Vec<Asset>, SwaplockApiError>
where
    L: IntoStringList,
{
    let value = session
        .database_call(
            "get_account_balances",
            json!([account_id, asset_ids.into_string_list()]),
        )
        .await?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_account_balances",
        message: error.to_string(),
    })
}

pub(super) async fn subscribe_account_balances_by_id(
    session: &mut GrapheneSession,
    account_id: String,
    asset_ids: Vec<String>,
) -> Result<AccountBalancesSubscription<'_>, SwaplockApiError> {
    session
        .database_call(
            "set_subscribe_callback",
            json!([ACCOUNT_BALANCES_CALLBACK_ID, false]),
        )
        .await?;
    let value = session
        .database_call("get_full_accounts", json!([[account_id], true]))
        .await?;
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

pub(crate) fn account_balances_from_full_accounts_value(
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

pub(crate) fn collect_account_balance_objects(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_account_balance_objects_from_full_accounts_response() {
        let asset_ids = vec!["1.3.0".to_string()];
        let balances = account_balances_from_full_accounts_value(
            "get_full_accounts",
            json!([["1.2.100", {"balances": [
                {"id": "2.5.1", "owner": "1.2.100", "asset_type": "1.3.0", "balance": 42, "maintenance_flag": false},
                {"id": "2.5.2", "owner": "1.2.100", "asset_type": "1.3.1", "balance": 7, "maintenance_flag": false}
            ]}]]),
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
    fn empty_asset_filter_parses_all_account_balance_objects_from_full_accounts_response() {
        let balances = account_balances_from_full_accounts_value(
            "get_full_accounts",
            json!([["1.2.100", {"balances": [
                {"id": "2.5.1", "owner": "1.2.100", "asset_type": "1.3.0", "balance": 42, "maintenance_flag": false},
                {"id": "2.5.2", "owner": "1.2.100", "asset_type": "1.3.1", "balance": 7, "maintenance_flag": false}
            ]}]]),
            "1.2.100",
            &[],
        )
        .unwrap();

        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0].asset_type.0, "1.3.0");
        assert_eq!(balances[1].asset_type.0, "1.3.1");
    }

    #[test]
    fn collects_only_matching_account_balance_notice_objects() {
        let asset_ids = vec!["1.3.0".to_string()];
        let balances = collect_account_balance_objects(
            "notice",
            &json!([[
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
    }

    #[test]
    fn empty_asset_filter_collects_all_account_balance_notice_objects_for_owner() {
        let balances = collect_account_balance_objects(
            "notice",
            &json!([[
                {"id": "2.5.1", "owner": "1.2.100", "asset_type": "1.3.0", "balance": 42, "maintenance_flag": false},
                {"id": "2.5.2", "owner": "1.2.100", "asset_type": "1.3.1", "balance": 7, "maintenance_flag": false},
                {"id": "2.5.3", "owner": "1.2.101", "asset_type": "1.3.0", "balance": 9, "maintenance_flag": false}
            ]]),
            "1.2.100",
            &[],
        )
        .unwrap();

        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0].id.0, "2.5.1");
        assert_eq!(balances[1].id.0, "2.5.2");
    }
}
