use std::convert::Infallible;

use open_graphene_sdk_core::TransactionHeader;
use open_graphene_sdk_operations::{
    build_limit_order_cancel_transaction_for, signed_transaction_broadcast_json,
    LimitOrderCancelAdapter, LimitOrderCancelChainTypes, LimitOrderCancelInput,
    SignedTransactionJsonParts,
};
use serde_json::{json, Value};
use thiserror::Error;

use crate::operation_builder_types::SwaplockOperationBuilderTypes;
use graphene_chain_swaplock_bindings::generated::operations::LimitOrderCancelOperation;
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction, Transaction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOrderCancelTransactionInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    pub fee_amount: i64,
    pub fee_asset_id: String,
    pub fee_paying_account_id: String,
    pub order_id: String,
}

impl LimitOrderCancelChainTypes for SwaplockOperationBuilderTypes {
    type LimitOrderCancelOperation = LimitOrderCancelOperation;

    fn limit_order_cancel_operation(
        fee: Self::Asset,
        fee_paying_account: Self::AccountId,
        order: Self::LimitOrderId,
        extensions: Self::FutureExtensions,
    ) -> Self::LimitOrderCancelOperation {
        LimitOrderCancelOperation {
            fee,
            fee_paying_account,
            order,
            extensions,
        }
    }

    fn operation_limit_order_cancel(operation: Self::LimitOrderCancelOperation) -> Self::Operation {
        Operation::LimitOrderCancelOperation(Box::new(operation))
    }
}

pub struct SwaplockLimitOrderCancelAdapter;

impl SwaplockLimitOrderCancelAdapter {
    pub fn build_limit_order_cancel_transaction(
        input: LimitOrderCancelInput,
    ) -> Result<Transaction, Infallible> {
        <Self as LimitOrderCancelAdapter>::build_limit_order_cancel_transaction(input)
    }
}

impl LimitOrderCancelAdapter for SwaplockLimitOrderCancelAdapter {
    type Transaction = Transaction;
    type Error = Infallible;

    fn build_limit_order_cancel_transaction(
        input: LimitOrderCancelInput,
    ) -> Result<Self::Transaction, Self::Error> {
        Ok(build_limit_order_cancel_transaction_for::<
            SwaplockOperationBuilderTypes,
        >(input))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LimitOrderCancelJsonError {
    #[error("signed limit-order-cancel JSON can only render limit_order_cancel operations")]
    UnsupportedOperation,
}

pub fn build_limit_order_cancel_transaction(
    input: LimitOrderCancelTransactionInput,
) -> Transaction {
    build_limit_order_cancel_transaction_for::<SwaplockOperationBuilderTypes>(
        LimitOrderCancelInput::new(
            TransactionHeader {
                ref_block_num: input.ref_block_num,
                ref_block_prefix: input.ref_block_prefix,
                expiration: input.expiration,
            },
            open_graphene_sdk_operations::FeeInput::new(input.fee_amount, input.fee_asset_id),
            input.fee_paying_account_id,
            input.order_id,
        ),
    )
}

pub fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<Value, LimitOrderCancelJsonError> {
    let operations = signed_transaction
        .operations
        .iter()
        .map(operation_json)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(signed_transaction_broadcast_json(
        SignedTransactionJsonParts {
            ref_block_num: signed_transaction.ref_block_num,
            ref_block_prefix: signed_transaction.ref_block_prefix,
            expiration: &signed_transaction.expiration,
            operations,
            signatures: signed_transaction
                .signatures
                .iter()
                .map(|signature| signature.0.as_slice())
                .collect(),
        },
    ))
}

fn operation_json(operation: &Operation) -> Result<Value, LimitOrderCancelJsonError> {
    let operation = match operation {
        Operation::LimitOrderCancelOperation(operation) => operation,
        _ => return Err(LimitOrderCancelJsonError::UnsupportedOperation),
    };

    Ok(json!([
        2,
        {
            "fee": asset_json(&operation.fee),
            "fee_paying_account": operation.fee_paying_account.0,
            "order": operation.order.0,
            "extensions": []
        }
    ]))
}

fn asset_json(asset: &Asset) -> Value {
    json!({
        "amount": asset.amount,
        "asset_id": asset.asset_id.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, LimitOrderId};
    use graphene_chain_swaplock_bindings::generated::operations::TransferOperation;
    use graphene_chain_swaplock_bindings::generated::static_variants::FutureExtensions;
    use graphene_chain_swaplock_bindings::generated::types::Signature;
    use graphene_chain_swaplock_bindings::generated::FcSerialize;

    fn input() -> LimitOrderCancelTransactionInput {
        LimitOrderCancelTransactionInput {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            fee_amount: 200_000,
            fee_asset_id: "1.3.0".to_string(),
            fee_paying_account_id: "1.2.100".to_string(),
            order_id: "1.7.123".to_string(),
        }
    }

    fn expected_transaction(input: LimitOrderCancelTransactionInput) -> Transaction {
        Transaction {
            ref_block_num: input.ref_block_num,
            ref_block_prefix: input.ref_block_prefix,
            expiration: input.expiration,
            operations: vec![Operation::LimitOrderCancelOperation(Box::new(
                LimitOrderCancelOperation {
                    fee: Asset::new(input.fee_amount, AssetId::new(input.fee_asset_id)),
                    fee_paying_account: AccountId::new(input.fee_paying_account_id),
                    order: LimitOrderId::new(input.order_id),
                    extensions: FutureExtensions::empty(),
                },
            ))],
            extensions: FutureExtensions::empty(),
        }
    }

    #[test]
    fn builds_limit_order_cancel_transaction_with_generated_swaplock_types() {
        let input = input();
        let transaction = build_limit_order_cancel_transaction(input.clone());
        let expected = expected_transaction(input);

        assert_eq!(
            transaction.to_fc_bytes().unwrap(),
            expected.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn common_limit_order_cancel_adapter_matches_manual_swaplock_builder() {
        let adapter_transaction =
            SwaplockLimitOrderCancelAdapter::build_limit_order_cancel_transaction(
                LimitOrderCancelInput {
                    header: open_graphene_sdk_core::TransactionHeader {
                        ref_block_num: 2,
                        ref_block_prefix: 3,
                        expiration: "2026-05-25T12:01:00".to_string(),
                    },
                    fee: open_graphene_sdk_operations::FeeInput::new(200_000, "1.3.0"),
                    fee_paying_account: open_graphene_sdk_operations::AccountRefInput::new(
                        "1.2.100",
                    ),
                    order: open_graphene_sdk_operations::LimitOrderRefInput::new("1.7.123"),
                },
            )
            .unwrap();
        let manual_transaction = build_limit_order_cancel_transaction(input());

        assert_eq!(
            adapter_transaction.to_fc_bytes().unwrap(),
            manual_transaction.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn renders_signed_limit_order_cancel_json_for_broadcast() {
        let mut transaction = build_limit_order_cancel_transaction(input());
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
                "operations": [[2, {
                    "fee": { "amount": 200000, "asset_id": "1.3.0" },
                    "fee_paying_account": "1.2.100",
                    "order": "1.7.123",
                    "extensions": []
                }]],
                "extensions": [],
                "signatures": ["1f".repeat(65)]
            })
        );
    }

    #[test]
    fn rejects_non_limit_order_cancel_operations() {
        let signed = SignedTransaction {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            operations: vec![Operation::TransferOperation(Box::new(TransferOperation {
                fee: Asset::new(200_000, AssetId::new("1.3.0")),
                from: AccountId::new("1.2.100"),
                to: AccountId::new("1.2.101"),
                amount: Asset::new(100_000, AssetId::new("1.3.0")),
                memo: None,
                extensions: FutureExtensions::empty(),
            }))],
            extensions: FutureExtensions::empty(),
            signatures: vec![],
        };

        assert_eq!(
            signed_transaction_json(&signed).unwrap_err(),
            LimitOrderCancelJsonError::UnsupportedOperation
        );
    }
}
