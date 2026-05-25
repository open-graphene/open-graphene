use std::error::Error;

use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction, Transaction};
use serde_json::Value;

use crate::database_api::required_fee;
use crate::rpc::GrapheneRpc;

pub fn apply_required_fee<F, E>(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    transaction: &mut Transaction,
    fee_asset_id: &str,
    max_fee: i64,
    renderer: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&SignedTransaction) -> Result<Value, E>,
    E: Error + 'static,
{
    let fee = required_fee(rpc, database_api_id, transaction, fee_asset_id, renderer)?;
    if fee.amount > max_fee {
        return Err(format!(
            "required fee {} exceeds max_fee {}; refusing to apply fee",
            fee.amount, max_fee
        )
        .into());
    }
    set_first_operation_fee(transaction, fee)
}

pub fn set_first_operation_fee(
    transaction: &mut Transaction,
    fee: Asset,
) -> Result<(), Box<dyn Error>> {
    let operation = transaction
        .operations
        .first_mut()
        .ok_or("transaction contains no operations")?;
    match operation {
        Operation::TransferOperation(operation) => operation.fee = fee,
        Operation::LimitOrderCreateOperation(operation) => operation.fee = fee,
        Operation::LimitOrderCancelOperation(operation) => operation.fee = fee,
        Operation::AccountCreateOperation(operation) => operation.fee = fee,
        Operation::AssetCreateOperation(operation) => operation.fee = fee,
        Operation::AssetIssueOperation(operation) => operation.fee = fee,
        _ => return Err("unsupported operation for fee injection".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::transfer::{build_transfer_transaction, TransferTransactionInput};
    use graphene_chain_swaplock_bindings::generated::ids::AssetId;
    use graphene_chain_swaplock_bindings::generated::static_variants::FutureExtensions;

    #[test]
    fn set_first_operation_fee_updates_supported_operation() {
        let mut transaction = fixture_transfer_transaction();

        set_first_operation_fee(
            &mut transaction,
            Asset::new(42, AssetId("1.3.0".to_string())),
        )
        .expect("transfer fee is supported");

        let Operation::TransferOperation(operation) = transaction.operations.first().unwrap()
        else {
            panic!("fixture should contain transfer operation");
        };
        assert_eq!(operation.fee.amount, 42);
        assert_eq!(operation.fee.asset_id.0, "1.3.0");
    }

    #[test]
    fn set_first_operation_fee_rejects_empty_transaction() {
        let mut transaction = Transaction {
            ref_block_num: 1,
            ref_block_prefix: 2,
            expiration: "2026-01-01T00:00:00".to_string(),
            operations: Vec::new(),
            extensions: FutureExtensions::empty(),
        };

        let err = set_first_operation_fee(
            &mut transaction,
            Asset::new(42, AssetId("1.3.0".to_string())),
        )
        .expect_err("empty operation list fails");

        assert_eq!(err.to_string(), "transaction contains no operations");
    }

    fn fixture_transfer_transaction() -> Transaction {
        build_transfer_transaction(TransferTransactionInput {
            ref_block_num: 1,
            ref_block_prefix: 2,
            expiration: "2026-01-01T00:00:00".to_string(),
            from_id: "1.2.100".to_string(),
            to_id: "1.2.101".to_string(),
            asset_id: "1.3.0".to_string(),
            amount: 1,
            fee_amount: 0,
            fee_asset_id: "1.3.0".to_string(),
        })
    }
}
