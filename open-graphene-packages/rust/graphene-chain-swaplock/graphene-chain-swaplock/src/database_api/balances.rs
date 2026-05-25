use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use serde_json::{json, Value};

use crate::rpc::GrapheneRpc;

pub fn account_balance(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
    asset_id: &str,
) -> Result<i64, Box<dyn Error>> {
    let balances = rpc.call_database(
        api_id,
        "get_account_balances",
        json!([account_id, [asset_id]]),
    )?;
    let balance = balances
        .as_array()
        .and_then(|values| values.first())
        .ok_or("get_account_balances returned no balance")?;
    if balance.get("asset_id").and_then(Value::as_str) != Some(asset_id) {
        return Err("get_account_balances returned unexpected asset_id".into());
    }
    balance
        .get("amount")
        .and_then(json_i64)
        .ok_or_else(|| "balance missing integer amount".into())
}

pub fn wait_for_balance_at_least(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
    asset_id: &str,
    minimum: i64,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..20 {
        let balance = account_balance(rpc, api_id, account_id, asset_id)?;
        if balance >= minimum {
            println!("Balance ok: {account_id} has {balance} {asset_id}");
            return Ok(());
        }
        sleep(Duration::from_millis(500));
    }
    Err(format!("balance for {account_id} {asset_id} did not reach {minimum}").into())
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
    fn json_i64_accepts_integer_or_string() {
        assert_eq!(json_i64(&json!(42)), Some(42));
        assert_eq!(json_i64(&json!("42")), Some(42));
        assert_eq!(json_i64(&json!("nope")), None);
    }
}
