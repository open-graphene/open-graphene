use std::convert::Infallible;

use open_graphene_sdk_core::TransactionHeader;
use open_graphene_sdk_operations::{
    build_limit_order_create_transaction_for, signed_transaction_broadcast_json,
    LimitOrderCreateAdapter, LimitOrderCreateChainTypes, LimitOrderCreateInput,
    SignedTransactionJsonParts,
};
use serde_json::{json, Value};
use thiserror::Error;

use crate::operation_builder_types::SwaplockOperationBuilderTypes;
use graphene_chain_swaplock_bindings::generated::operations::LimitOrderCreateOperation;
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction, Transaction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOrderCreateTransactionInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub transaction_expiration: String,
    pub fee_amount: i64,
    pub fee_asset_id: String,
    pub seller_id: String,
    pub amount_to_sell_amount: i64,
    pub amount_to_sell_asset_id: String,
    pub min_to_receive_amount: i64,
    pub min_to_receive_asset_id: String,
    pub order_expiration: String,
    pub fill_or_kill: bool,
}

impl LimitOrderCreateChainTypes for SwaplockOperationBuilderTypes {
    type LimitOrderCreateOperation = LimitOrderCreateOperation;

    fn limit_order_create_operation(
        fee: Self::Asset,
        seller: Self::AccountId,
        amount_to_sell: Self::Asset,
        min_to_receive: Self::Asset,
        expiration: String,
        fill_or_kill: bool,
        extensions: Self::FutureExtensions,
    ) -> Self::LimitOrderCreateOperation {
        LimitOrderCreateOperation {
            fee,
            seller,
            amount_to_sell,
            min_to_receive,
            expiration,
            fill_or_kill,
            extensions,
        }
    }

    fn operation_limit_order_create(operation: Self::LimitOrderCreateOperation) -> Self::Operation {
        Operation::limit_order_create(operation)
    }
}

pub struct SwaplockLimitOrderCreateAdapter;

impl SwaplockLimitOrderCreateAdapter {
    pub fn build_limit_order_create_transaction(
        input: LimitOrderCreateInput,
    ) -> Result<Transaction, Infallible> {
        <Self as LimitOrderCreateAdapter>::build_limit_order_create_transaction(input)
    }
}

impl LimitOrderCreateAdapter for SwaplockLimitOrderCreateAdapter {
    type Transaction = Transaction;
    type Error = Infallible;

    fn build_limit_order_create_transaction(
        input: LimitOrderCreateInput,
    ) -> Result<Self::Transaction, Self::Error> {
        Ok(build_limit_order_create_transaction_for::<
            SwaplockOperationBuilderTypes,
        >(input))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LimitOrderCreateJsonError {
    #[error("signed limit-order-create JSON can only render limit_order_create operations")]
    UnsupportedOperation,
}

pub fn build_limit_order_create_transaction(
    input: LimitOrderCreateTransactionInput,
) -> Transaction {
    build_limit_order_create_transaction_for::<SwaplockOperationBuilderTypes>(
        LimitOrderCreateInput::new(
            TransactionHeader {
                ref_block_num: input.ref_block_num,
                ref_block_prefix: input.ref_block_prefix,
                expiration: input.transaction_expiration,
            },
            open_graphene_sdk_operations::FeeInput::new(input.fee_amount, input.fee_asset_id),
            input.seller_id,
            open_graphene_sdk_operations::AssetAmountInput::new(
                input.amount_to_sell_amount,
                input.amount_to_sell_asset_id,
            ),
            open_graphene_sdk_operations::AssetAmountInput::new(
                input.min_to_receive_amount,
                input.min_to_receive_asset_id,
            ),
            input.order_expiration,
            input.fill_or_kill,
        ),
    )
}

pub fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<Value, LimitOrderCreateJsonError> {
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

fn operation_json(operation: &Operation) -> Result<Value, LimitOrderCreateJsonError> {
    let operation = operation
        .as_limit_order_create()
        .ok_or(LimitOrderCreateJsonError::UnsupportedOperation)?;

    Ok(json!([
        1,
        {
            "fee": asset_json(&operation.fee),
            "seller": operation.seller.0,
            "amount_to_sell": asset_json(&operation.amount_to_sell),
            "min_to_receive": asset_json(&operation.min_to_receive),
            "expiration": operation.expiration,
            "fill_or_kill": operation.fill_or_kill,
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
    use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
    use graphene_chain_swaplock_bindings::generated::operations::TransferOperation;
    use graphene_chain_swaplock_bindings::generated::static_variants::FutureExtensions;
    use graphene_chain_swaplock_bindings::generated::types::Signature;
    use graphene_chain_swaplock_bindings::generated::FcSerialize;

    fn input() -> LimitOrderCreateTransactionInput {
        LimitOrderCreateTransactionInput {
            ref_block_num: 2,
            ref_block_prefix: 3,
            transaction_expiration: "2026-05-25T12:01:00".to_string(),
            fee_amount: 200_000,
            fee_asset_id: "1.3.0".to_string(),
            seller_id: "1.2.100".to_string(),
            amount_to_sell_amount: 100_000,
            amount_to_sell_asset_id: "1.3.1".to_string(),
            min_to_receive_amount: 250_000,
            min_to_receive_asset_id: "1.3.0".to_string(),
            order_expiration: "2026-05-26T12:01:00".to_string(),
            fill_or_kill: false,
        }
    }

    fn expected_transaction(input: LimitOrderCreateTransactionInput) -> Transaction {
        Transaction {
            ref_block_num: input.ref_block_num,
            ref_block_prefix: input.ref_block_prefix,
            expiration: input.transaction_expiration,
            operations: vec![Operation::LimitOrderCreateOperation(Box::new(
                LimitOrderCreateOperation {
                    fee: Asset::new(input.fee_amount, AssetId::new(input.fee_asset_id)),
                    seller: AccountId::new(input.seller_id),
                    amount_to_sell: Asset::new(
                        input.amount_to_sell_amount,
                        AssetId::new(input.amount_to_sell_asset_id),
                    ),
                    min_to_receive: Asset::new(
                        input.min_to_receive_amount,
                        AssetId::new(input.min_to_receive_asset_id),
                    ),
                    expiration: input.order_expiration,
                    fill_or_kill: input.fill_or_kill,
                    extensions: FutureExtensions::empty(),
                },
            ))],
            extensions: FutureExtensions::empty(),
        }
    }

    #[test]
    fn builds_limit_order_create_transaction_with_generated_swaplock_types() {
        let input = input();
        let transaction = build_limit_order_create_transaction(input.clone());
        let expected = expected_transaction(input);

        assert_eq!(
            transaction.to_fc_bytes().unwrap(),
            expected.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn common_limit_order_create_adapter_matches_manual_swaplock_builder() {
        let adapter_transaction =
            SwaplockLimitOrderCreateAdapter::build_limit_order_create_transaction(
                LimitOrderCreateInput::new(
                    open_graphene_sdk_core::TransactionHeader {
                        ref_block_num: 2,
                        ref_block_prefix: 3,
                        expiration: "2026-05-25T12:01:00".to_string(),
                    },
                    open_graphene_sdk_operations::FeeInput::new(200_000, "1.3.0"),
                    "1.2.100",
                    open_graphene_sdk_operations::AssetAmountInput::new(100_000, "1.3.1"),
                    open_graphene_sdk_operations::AssetAmountInput::new(250_000, "1.3.0"),
                    "2026-05-26T12:01:00",
                    false,
                ),
            )
            .unwrap();
        let manual_transaction = build_limit_order_create_transaction(input());

        assert_eq!(
            adapter_transaction.to_fc_bytes().unwrap(),
            manual_transaction.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn renders_signed_limit_order_create_json_for_broadcast() {
        let mut transaction = build_limit_order_create_transaction(input());
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
                "operations": [[1, {
                    "fee": { "amount": 200000, "asset_id": "1.3.0" },
                    "seller": "1.2.100",
                    "amount_to_sell": { "amount": 100000, "asset_id": "1.3.1" },
                    "min_to_receive": { "amount": 250000, "asset_id": "1.3.0" },
                    "expiration": "2026-05-26T12:01:00",
                    "fill_or_kill": false,
                    "extensions": []
                }]],
                "extensions": [],
                "signatures": ["1f".repeat(65)]
            })
        );
    }

    #[test]
    fn rejects_non_limit_order_create_operations() {
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
            LimitOrderCreateJsonError::UnsupportedOperation
        );
    }
}
