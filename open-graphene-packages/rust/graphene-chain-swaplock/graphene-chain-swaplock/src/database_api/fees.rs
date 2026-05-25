use std::error::Error;

use graphene_chain_swaplock_bindings::generated::ids::AssetId;
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction, Transaction};
use serde_json::{json, Value};

use crate::rpc::GrapheneRpc;

pub fn required_fee<F, E>(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    transaction: &Transaction,
    fee_asset_id: &str,
    renderer: F,
) -> Result<Asset, Box<dyn Error>>
where
    F: Fn(&SignedTransaction) -> Result<Value, E>,
    E: Error + 'static,
{
    let unsigned = SignedTransaction {
        ref_block_num: transaction.ref_block_num,
        ref_block_prefix: transaction.ref_block_prefix,
        expiration: transaction.expiration.clone(),
        operations: transaction.operations.clone(),
        extensions: transaction.extensions.clone(),
        signatures: Vec::new(),
    };
    let tx_json = renderer(&unsigned).map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
    let op_json = tx_json
        .get("operations")
        .and_then(Value::as_array)
        .and_then(|operations| operations.first())
        .cloned()
        .ok_or("transaction JSON missing first operation")?;
    let result = rpc.call_database(
        database_api_id,
        "get_required_fees",
        json!([[op_json], fee_asset_id]),
    )?;
    parse_asset(
        result
            .as_array()
            .and_then(|values| values.first())
            .ok_or("get_required_fees returned no fee")?,
    )
}

fn parse_asset(value: &Value) -> Result<Asset, Box<dyn Error>> {
    Ok(Asset {
        amount: value
            .get("amount")
            .and_then(json_i64)
            .ok_or("asset missing integer amount")?,
        asset_id: AssetId(
            value
                .get("asset_id")
                .and_then(Value::as_str)
                .ok_or("asset missing asset_id")?
                .to_string(),
        ),
    })
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
    fn parse_asset_accepts_integer_or_string_amount() {
        assert_eq!(
            parse_asset(&json!({"amount": 7, "asset_id": "1.3.0"}))
                .unwrap()
                .amount,
            7
        );
        assert_eq!(
            parse_asset(&json!({"amount": "8", "asset_id": "1.3.0"}))
                .unwrap()
                .amount,
            8
        );
    }

    #[test]
    fn parse_asset_rejects_missing_asset_id() {
        assert!(parse_asset(&json!({"amount": 7})).is_err());
    }
}
