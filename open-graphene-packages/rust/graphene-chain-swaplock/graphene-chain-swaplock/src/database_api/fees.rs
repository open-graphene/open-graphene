use std::error::Error;

use graphene_chain_swaplock_bindings::generated::ids::AssetId;
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction, Transaction};
use open_graphene_sdk_core::{AssetAmount, AssetIdRef};
use serde_json::Value;

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
    ensure_database_api_id(rpc, database_api_id)?;
    let fee_asset_id = AssetIdRef::parse(fee_asset_id)?;
    let fee = open_graphene_sdk_live::required_fee_for_operation_json(
        rpc.session_mut(),
        op_json,
        &fee_asset_id,
    )?;
    Ok(asset_from_sdk_amount(fee))
}

fn asset_from_sdk_amount(value: AssetAmount) -> Asset {
    Asset {
        amount: value.amount,
        asset_id: AssetId(value.asset_id.to_string()),
    }
}

fn ensure_database_api_id(rpc: &mut GrapheneRpc, api_id: u64) -> Result<(), Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != api_id {
        return Err(format!("database API id mismatch: expected {expected}, got {api_id}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_from_sdk_amount_preserves_amount_and_asset_id() {
        let asset = asset_from_sdk_amount(AssetAmount::new(8, AssetIdRef::parse("1.3.0").unwrap()));

        assert_eq!(asset.amount, 8);
        assert_eq!(asset.asset_id.0, "1.3.0");
    }
}
