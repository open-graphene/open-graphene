use graphene_chain_swaplock_bindings::generated::LimitOrderObject;
use open_graphene_transport::{CallbackId, GrapheneSession, JsonRpcInbound};
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::constants::ACCOUNT_ORDERS_CALLBACK_ID;
use super::error::unexpected_response;

pub struct AccountOrdersByIdRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_id: String,
}

pub struct AccountOrdersSubscription<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    initial: Vec<LimitOrderObject>,
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
            let notice = self.session.next_notice().await?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != CallbackId::new(ACCOUNT_ORDERS_CALLBACK_ID) {
                continue;
            }

            let updates = collect_account_order_objects("notice", &payload, &self.account_id)?;
            if !updates.is_empty() {
                return Ok(updates);
            }
        }
    }
}

pub(super) async fn get_account_orders_by_id(
    session: &mut GrapheneSession,
    account_id: &str,
) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
    let value = session
        .database_call("get_full_accounts", json!([[account_id], false]))
        .await?;
    account_orders_from_full_accounts_value("get_full_accounts", value, account_id)
}

pub(super) async fn subscribe_account_orders_by_id(
    session: &mut GrapheneSession,
    account_id: String,
) -> Result<AccountOrdersSubscription<'_>, SwaplockApiError> {
    session
        .database_call(
            "set_subscribe_callback",
            json!([ACCOUNT_ORDERS_CALLBACK_ID, false]),
        )
        .await?;
    let value = session
        .database_call("get_full_accounts", json!([[account_id], true]))
        .await?;
    let initial = account_orders_from_full_accounts_value("get_full_accounts", value, &account_id)?;

    Ok(AccountOrdersSubscription {
        session,
        account_id,
        initial,
    })
}

pub(crate) fn account_orders_from_full_accounts_value(
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

pub(crate) fn collect_account_order_objects(
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

#[cfg(test)]
mod tests {
    use super::*;

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
