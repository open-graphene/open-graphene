use std::convert::Infallible;

use open_graphene_sdk_core::TransactionHeader;
use open_graphene_sdk_operations::{
    build_transfer_transaction_for, TransferAdapter, TransferChainTypes, TransferInput,
};
use serde_json::{json, Value};
use thiserror::Error;

use crate::generated::ids::{AccountId, AssetId};
use crate::generated::operations::TransferOperation;
use crate::generated::static_variants::{FutureExtensions, Operation};
use crate::generated::types::{Asset, SignedTransaction, Transaction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferTransactionInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    pub from_id: String,
    pub to_id: String,
    pub asset_id: String,
    pub amount: i64,
    pub fee_amount: i64,
    pub fee_asset_id: String,
}

pub struct BitSharesTransferTypes;

impl From<String> for AccountId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<String> for AssetId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl TransferChainTypes for BitSharesTransferTypes {
    type Transaction = Transaction;
    type Operation = Operation;
    type TransferOperation = TransferOperation;
    type Asset = Asset;
    type AccountId = AccountId;
    type AssetId = AssetId;
    type FutureExtensions = FutureExtensions;

    fn asset(amount: i64, asset_id: Self::AssetId) -> Self::Asset {
        Asset { amount, asset_id }
    }

    fn empty_extensions() -> Self::FutureExtensions {
        FutureExtensions::VoidT(Box::new(()))
    }

    fn transfer_operation_without_memo(
        fee: Self::Asset,
        from: Self::AccountId,
        to: Self::AccountId,
        amount: Self::Asset,
        extensions: Self::FutureExtensions,
    ) -> Self::TransferOperation {
        TransferOperation {
            fee,
            from,
            to,
            amount,
            memo: None,
            extensions,
        }
    }

    fn operation_transfer(operation: Self::TransferOperation) -> Self::Operation {
        Operation::TransferOperation(Box::new(operation))
    }

    fn transaction(
        header: TransactionHeader,
        operations: Vec<Self::Operation>,
        extensions: Self::FutureExtensions,
    ) -> Self::Transaction {
        Transaction {
            ref_block_num: header.ref_block_num,
            ref_block_prefix: header.ref_block_prefix,
            expiration: header.expiration,
            operations,
            extensions,
        }
    }
}

pub struct BitSharesTransferAdapter;

impl TransferAdapter for BitSharesTransferAdapter {
    type Transaction = Transaction;
    type Error = Infallible;

    fn build_transfer_transaction(input: TransferInput) -> Result<Self::Transaction, Self::Error> {
        Ok(build_transfer_transaction_for::<BitSharesTransferTypes>(
            input,
        ))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransferJsonError {
    #[error("signed transfer JSON can only render transfer operations")]
    UnsupportedOperation,
}

pub fn build_transfer_transaction(input: TransferTransactionInput) -> Transaction {
    build_transfer_transaction_for::<BitSharesTransferTypes>(TransferInput::new(
        TransactionHeader {
            ref_block_num: input.ref_block_num,
            ref_block_prefix: input.ref_block_prefix,
            expiration: input.expiration,
        },
        input.from_id,
        input.to_id,
        input.amount,
        input.asset_id,
        input.fee_amount,
        input.fee_asset_id,
    ))
}

pub fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<Value, TransferJsonError> {
    Ok(json!({
        "ref_block_num": signed_transaction.ref_block_num,
        "ref_block_prefix": signed_transaction.ref_block_prefix,
        "expiration": signed_transaction.expiration,
        "operations": signed_transaction
            .operations
            .iter()
            .map(operation_json)
            .collect::<Result<Vec<_>, _>>()?,
        "extensions": [],
        "signatures": signed_transaction
            .signatures
            .iter()
            .map(|signature| hex(&signature.0))
            .collect::<Vec<_>>(),
    }))
}

fn operation_json(operation: &Operation) -> Result<Value, TransferJsonError> {
    match operation {
        Operation::TransferOperation(operation) => Ok(json!([
            0,
            {
                "fee": asset_json(&operation.fee),
                "from": operation.from.0,
                "to": operation.to.0,
                "amount": asset_json(&operation.amount),
                "memo": null,
                "extensions": []
            }
        ])),
        _ => Err(TransferJsonError::UnsupportedOperation),
    }
}

fn asset_json(asset: &Asset) -> Value {
    json!({
        "amount": asset.amount,
        "asset_id": asset.asset_id.0,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::types::Signature;
    use crate::generated::FcSerialize;

    #[test]
    fn builds_transfer_transaction_with_generated_bitshares_types() {
        let transaction = build_transfer_transaction(TransferTransactionInput {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            from_id: "1.2.100".to_string(),
            to_id: "1.2.0".to_string(),
            asset_id: "1.3.0".to_string(),
            amount: 100_000,
            fee_amount: 200_000,
            fee_asset_id: "1.3.0".to_string(),
        });

        let expected = Transaction {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            operations: vec![Operation::TransferOperation(Box::new(TransferOperation {
                fee: Asset {
                    amount: 200_000,
                    asset_id: AssetId("1.3.0".to_string()),
                },
                from: AccountId("1.2.100".to_string()),
                to: AccountId("1.2.0".to_string()),
                amount: Asset {
                    amount: 100_000,
                    asset_id: AssetId("1.3.0".to_string()),
                },
                memo: None,
                extensions: FutureExtensions::VoidT(Box::new(())),
            }))],
            extensions: FutureExtensions::VoidT(Box::new(())),
        };

        assert_eq!(
            transaction.to_fc_bytes().unwrap(),
            expected.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn common_transfer_adapter_matches_manual_bitshares_builder() {
        let adapter_transaction =
            BitSharesTransferAdapter::build_transfer_transaction(TransferInput {
                header: open_graphene_sdk_core::TransactionHeader {
                    ref_block_num: 2,
                    ref_block_prefix: 3,
                    expiration: "2026-05-25T12:01:00".to_string(),
                },
                from: open_graphene_sdk_operations::AccountRefInput::new("1.2.100"),
                to: open_graphene_sdk_operations::AccountRefInput::new("1.2.0"),
                amount: open_graphene_sdk_operations::AssetAmountInput::new(100_000, "1.3.0"),
                fee: open_graphene_sdk_operations::FeeInput::new(200_000, "1.3.0"),
            })
            .unwrap();
        let manual_transaction = build_transfer_transaction(TransferTransactionInput {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            from_id: "1.2.100".to_string(),
            to_id: "1.2.0".to_string(),
            asset_id: "1.3.0".to_string(),
            amount: 100_000,
            fee_amount: 200_000,
            fee_asset_id: "1.3.0".to_string(),
        });

        assert_eq!(
            adapter_transaction.to_fc_bytes().unwrap(),
            manual_transaction.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn renders_signed_transfer_json_for_broadcast() {
        let mut transaction = build_transfer_transaction(TransferTransactionInput {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            from_id: "1.2.100".to_string(),
            to_id: "1.2.0".to_string(),
            asset_id: "1.3.0".to_string(),
            amount: 100_000,
            fee_amount: 200_000,
            fee_asset_id: "1.3.0".to_string(),
        });
        let signed = SignedTransaction {
            ref_block_num: transaction.ref_block_num,
            ref_block_prefix: transaction.ref_block_prefix,
            expiration: transaction.expiration,
            operations: std::mem::take(&mut transaction.operations),
            extensions: transaction.extensions,
            signatures: vec![Signature(vec![0x1f; 65])],
        };

        assert_eq!(
            signed_transaction_json(&signed).unwrap(),
            json!({
                "ref_block_num": 2,
                "ref_block_prefix": 3,
                "expiration": "2026-05-25T12:01:00",
                "operations": [[0, {
                    "fee": { "amount": 200000, "asset_id": "1.3.0" },
                    "from": "1.2.100",
                    "to": "1.2.0",
                    "amount": { "amount": 100000, "asset_id": "1.3.0" },
                    "memo": null,
                    "extensions": []
                }]],
                "extensions": [],
                "signatures": ["1f".repeat(65)]
            })
        );
    }
}
