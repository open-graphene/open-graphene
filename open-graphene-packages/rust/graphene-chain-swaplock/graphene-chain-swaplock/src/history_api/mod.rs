use std::error::Error;
use std::time::Duration;

use serde_json::{json, Value};

use crate::rpc::GrapheneRpc;

pub struct AccountHistoryQuery<'a> {
    pub account_id: &'a str,
    pub stop: &'a str,
    pub limit: u64,
    pub start: &'a str,
}

impl<'a> AccountHistoryQuery<'a> {
    pub fn recent(account_id: &'a str) -> Self {
        Self {
            account_id,
            stop: "1.11.0",
            limit: 20,
            start: "1.11.0",
        }
    }

    pub fn params(&self) -> Value {
        json!([self.account_id, self.stop, self.limit, self.start])
    }
}

pub struct HistoryPollConfig {
    pub attempts: u64,
    pub delay: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryConfirmation {
    pub id: String,
    pub block_num: u64,
    pub trx_in_block: Option<u64>,
    pub op_in_trx: Option<u64>,
    pub virtual_op: Option<u64>,
}

pub struct AccountCreateConfirmationCriteria<'a> {
    pub account_name: &'a str,
    pub min_block_num: u64,
}

pub struct TransferConfirmationCriteria<'a> {
    pub from_id: &'a str,
    pub to_id: &'a str,
    pub amount: i64,
    pub asset_id: &'a str,
    pub fee_amount: i64,
    pub min_block_num: u64,
}

pub fn get_account_history(
    rpc: &mut GrapheneRpc,
    history_api_id: u64,
    query: &AccountHistoryQuery<'_>,
) -> Result<Value, Box<dyn Error>> {
    rpc.call_history(history_api_id, "get_account_history", query.params())
}

pub fn wait_for_account_history_confirmation<F>(
    rpc: &mut GrapheneRpc,
    history_api_id: u64,
    query: &AccountHistoryQuery<'_>,
    poll: &HistoryPollConfig,
    matcher: F,
) -> Result<Option<HistoryConfirmation>, Box<dyn Error>>
where
    F: Fn(&Value) -> bool,
{
    for attempt in 0..poll.attempts {
        let history = get_account_history(rpc, history_api_id, query)?;
        if let Some(confirmation) = find_account_history_confirmation(&history, &matcher)? {
            return Ok(Some(confirmation));
        }
        if attempt + 1 < poll.attempts {
            std::thread::sleep(poll.delay);
        }
    }

    Ok(None)
}

pub fn find_account_history_confirmation<F>(
    history: &Value,
    matcher: F,
) -> Result<Option<HistoryConfirmation>, Box<dyn Error>>
where
    F: Fn(&Value) -> bool,
{
    let Some(entries) = history.as_array() else {
        return Err("get_account_history result is not an array".into());
    };
    for entry in entries {
        if matcher(entry) {
            return Ok(Some(HistoryConfirmation {
                id: entry
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or("history entry missing id")?
                    .to_string(),
                block_num: entry
                    .get("block_num")
                    .and_then(Value::as_u64)
                    .ok_or("history entry missing block_num")?,
                trx_in_block: entry.get("trx_in_block").and_then(Value::as_u64),
                op_in_trx: entry.get("op_in_trx").and_then(Value::as_u64),
                virtual_op: entry.get("virtual_op").and_then(Value::as_u64),
            }));
        }
    }
    Ok(None)
}

pub fn history_entry_matches_account_create(
    entry: &Value,
    criteria: &AccountCreateConfirmationCriteria<'_>,
) -> bool {
    if !history_entry_matches_min_block(entry, criteria.min_block_num) {
        return false;
    }
    let Some(payload) = history_entry_payload_for_operation(entry, 5) else {
        return false;
    };
    payload.get("name").and_then(Value::as_str) == Some(criteria.account_name)
}

pub fn history_entry_matches_transfer(
    entry: &Value,
    criteria: &TransferConfirmationCriteria<'_>,
) -> bool {
    if !history_entry_matches_min_block(entry, criteria.min_block_num) {
        return false;
    }
    let Some(payload) = history_entry_payload_for_operation(entry, 0) else {
        return false;
    };
    payload.get("from").and_then(Value::as_str) == Some(criteria.from_id)
        && payload.get("to").and_then(Value::as_str) == Some(criteria.to_id)
        && asset_value_matches(payload.get("amount"), criteria.amount, criteria.asset_id)
        && asset_value_matches(payload.get("fee"), criteria.fee_amount, criteria.asset_id)
}

fn history_entry_matches_min_block(entry: &Value, min_block_num: u64) -> bool {
    entry.get("block_num").and_then(Value::as_u64) >= Some(min_block_num)
}

fn history_entry_payload_for_operation(entry: &Value, operation_id: u64) -> Option<&Value> {
    let operation = entry.get("op").and_then(Value::as_array)?;
    if operation.first().and_then(Value::as_u64) != Some(operation_id) {
        return None;
    }
    operation.get(1)
}

fn asset_value_matches(value: Option<&Value>, amount: i64, asset_id: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    value.get("amount").and_then(json_i64) == Some(amount)
        && value.get("asset_id").and_then(Value::as_str) == Some(asset_id)
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_account_history_query_uses_existing_wire_shape() {
        assert_eq!(
            AccountHistoryQuery::recent("1.2.100").params(),
            json!(["1.2.100", "1.11.0", 20, "1.11.0"])
        );
    }

    #[test]
    fn account_history_confirmation_matches_transfer_fields() {
        let history = json!([
            {
                "id": "1.11.22",
                "block_num": 123,
                "trx_in_block": 1,
                "op_in_trx": 0,
                "virtual_op": 0,
                "op": [0, {
                    "fee": { "amount": 10, "asset_id": "1.3.0" },
                    "from": "1.2.100",
                    "to": "1.2.0",
                    "amount": { "amount": 100000, "asset_id": "1.3.0" },
                    "memo": null,
                    "extensions": []
                }]
            }
        ]);
        let criteria = TransferConfirmationCriteria {
            from_id: "1.2.100",
            to_id: "1.2.0",
            amount: 100000,
            asset_id: "1.3.0",
            fee_amount: 10,
            min_block_num: 123,
        };
        let confirmation = find_account_history_confirmation(&history, |entry| {
            history_entry_matches_transfer(entry, &criteria)
        })
        .unwrap()
        .expect("matching transfer is confirmed");

        assert_eq!(confirmation.id, "1.11.22");
        assert_eq!(confirmation.block_num, 123);
        assert_eq!(confirmation.trx_in_block, Some(1));
        assert_eq!(confirmation.op_in_trx, Some(0));
        assert_eq!(confirmation.virtual_op, Some(0));
    }

    #[test]
    fn account_history_confirmation_rejects_wrong_transfer_amount() {
        let history = json!([
            {
                "id": "1.11.22",
                "block_num": 123,
                "trx_in_block": 1,
                "op_in_trx": 0,
                "virtual_op": 0,
                "op": [0, {
                    "fee": { "amount": 10, "asset_id": "1.3.0" },
                    "from": "1.2.100",
                    "to": "1.2.0",
                    "amount": { "amount": 1, "asset_id": "1.3.0" },
                    "memo": null,
                    "extensions": []
                }]
            }
        ]);
        let criteria = TransferConfirmationCriteria {
            from_id: "1.2.100",
            to_id: "1.2.0",
            amount: 100000,
            asset_id: "1.3.0",
            fee_amount: 10,
            min_block_num: 123,
        };

        assert!(find_account_history_confirmation(&history, |entry| {
            history_entry_matches_transfer(entry, &criteria)
        })
        .unwrap()
        .is_none());
    }

    #[test]
    fn account_history_confirmation_matches_account_create_name() {
        let history = json!([
            {
                "id": "1.11.23",
                "block_num": 123,
                "trx_in_block": 1,
                "op_in_trx": 0,
                "virtual_op": 0,
                "op": [5, { "name": "og-test-123" }]
            }
        ]);
        let criteria = AccountCreateConfirmationCriteria {
            account_name: "og-test-123",
            min_block_num: 123,
        };
        let confirmation = find_account_history_confirmation(&history, |entry| {
            history_entry_matches_account_create(entry, &criteria)
        })
        .unwrap()
        .expect("matching account_create is confirmed");

        assert_eq!(confirmation.id, "1.11.23");
        assert_eq!(confirmation.block_num, 123);
    }

    #[test]
    fn account_history_confirmation_rejects_non_array_history() {
        let err = find_account_history_confirmation(&json!({}), |_| true)
            .expect_err("non-array history is rejected");
        assert_eq!(
            err.to_string(),
            "get_account_history result is not an array"
        );
    }
}
