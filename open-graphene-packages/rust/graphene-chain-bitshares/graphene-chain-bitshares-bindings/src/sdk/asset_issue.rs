use std::convert::Infallible;

use open_graphene_sdk_core::TransactionHeader;
use open_graphene_sdk_operations::{
    AssetIssueAdapter, AssetIssueChainTypes, AssetIssueInput, build_asset_issue_transaction_for,
};
use serde_json::{Value, json};
use thiserror::Error;

use crate::generated::operations::AssetIssueOperation;
use crate::generated::static_variants::Operation;
use crate::generated::types::{Asset, SignedTransaction, Transaction};
use crate::sdk::operation_builder_types::BitSharesOperationBuilderTypes;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIssueTransactionInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    pub issuer_id: String,
    pub issue_to_account_id: String,
    pub asset_id: String,
    pub amount: i64,
    pub fee_amount: i64,
    pub fee_asset_id: String,
}

#[deprecated(note = "use BitSharesOperationBuilderTypes instead")]
pub type BitSharesAssetIssueTypes = BitSharesOperationBuilderTypes;

impl AssetIssueChainTypes for BitSharesOperationBuilderTypes {
    type AssetIssueOperation = AssetIssueOperation;

    fn asset_issue_operation(
        fee: Self::Asset,
        issuer: Self::AccountId,
        asset_to_issue: Self::Asset,
        issue_to_account: Self::AccountId,
        extensions: Self::FutureExtensions,
    ) -> Self::AssetIssueOperation {
        AssetIssueOperation {
            fee,
            issuer,
            asset_to_issue,
            issue_to_account,
            memo: None,
            extensions,
        }
    }

    fn operation_asset_issue(operation: Self::AssetIssueOperation) -> Self::Operation {
        Operation::asset_issue(operation)
    }
}

pub struct BitSharesAssetIssueAdapter;

impl AssetIssueAdapter for BitSharesAssetIssueAdapter {
    type Transaction = Transaction;
    type Error = Infallible;

    fn build_asset_issue_transaction(
        input: AssetIssueInput,
    ) -> Result<Self::Transaction, Self::Error> {
        Ok(build_asset_issue_transaction_for::<
            BitSharesOperationBuilderTypes,
        >(input))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssetIssueJsonError {
    #[error("signed asset-issue JSON can only render asset_issue operations")]
    UnsupportedOperation,
    #[error("asset_issue broadcast JSON does not support memo yet")]
    UnsupportedMemo,
}

pub fn build_asset_issue_transaction(input: AssetIssueTransactionInput) -> Transaction {
    build_asset_issue_transaction_for::<BitSharesOperationBuilderTypes>(AssetIssueInput::new(
        TransactionHeader {
            ref_block_num: input.ref_block_num,
            ref_block_prefix: input.ref_block_prefix,
            expiration: input.expiration,
        },
        open_graphene_sdk_operations::FeeInput::new(input.fee_amount, input.fee_asset_id),
        input.issuer_id,
        input.issue_to_account_id,
        input.amount,
        input.asset_id,
    ))
}

pub fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<Value, AssetIssueJsonError> {
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

fn operation_json(operation: &Operation) -> Result<Value, AssetIssueJsonError> {
    let operation = operation
        .as_asset_issue()
        .ok_or(AssetIssueJsonError::UnsupportedOperation)?;

    if operation.memo.is_some() {
        return Err(AssetIssueJsonError::UnsupportedMemo);
    }

    Ok(json!([
        14,
        {
            "fee": asset_json(&operation.fee),
            "issuer": operation.issuer.0,
            "asset_to_issue": asset_json(&operation.asset_to_issue),
            "issue_to_account": operation.issue_to_account.0,
            "memo": null,
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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::FcSerialize;
    use crate::generated::ids::{AccountId, AssetId};
    use crate::generated::operations::TransferOperation;
    use crate::generated::static_variants::FutureExtensions;
    use crate::generated::types::{MemoData, Signature};

    fn input() -> AssetIssueTransactionInput {
        AssetIssueTransactionInput {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            issuer_id: "1.2.100".to_string(),
            issue_to_account_id: "1.2.101".to_string(),
            asset_id: "1.3.1".to_string(),
            amount: 100_000,
            fee_amount: 200_000,
            fee_asset_id: "1.3.0".to_string(),
        }
    }

    fn expected_transaction(input: AssetIssueTransactionInput) -> Transaction {
        Transaction {
            ref_block_num: input.ref_block_num,
            ref_block_prefix: input.ref_block_prefix,
            expiration: input.expiration,
            operations: vec![Operation::AssetIssueOperation(Box::new(
                AssetIssueOperation {
                    fee: Asset::new(input.fee_amount, AssetId::new(input.fee_asset_id)),
                    issuer: AccountId(input.issuer_id),
                    asset_to_issue: Asset::new(input.amount, AssetId::new(input.asset_id)),
                    issue_to_account: AccountId(input.issue_to_account_id),
                    memo: None,
                    extensions: FutureExtensions::empty(),
                },
            ))],
            extensions: FutureExtensions::empty(),
        }
    }

    #[test]
    fn builds_asset_issue_transaction_with_generated_bitshares_types() {
        let input = input();
        let transaction = build_asset_issue_transaction(input.clone());
        let expected = expected_transaction(input);

        assert_eq!(
            transaction.to_fc_bytes().unwrap(),
            expected.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn common_asset_issue_adapter_matches_manual_bitshares_builder() {
        let adapter_transaction =
            BitSharesAssetIssueAdapter::build_asset_issue_transaction(AssetIssueInput {
                header: open_graphene_sdk_core::TransactionHeader {
                    ref_block_num: 2,
                    ref_block_prefix: 3,
                    expiration: "2026-05-25T12:01:00".to_string(),
                },
                fee: open_graphene_sdk_operations::FeeInput::new(200_000, "1.3.0"),
                issuer: open_graphene_sdk_operations::AccountRefInput::new("1.2.100"),
                issue_to_account: open_graphene_sdk_operations::AccountRefInput::new("1.2.101"),
                asset_to_issue: open_graphene_sdk_operations::AssetAmountInput::new(
                    100_000, "1.3.1",
                ),
            })
            .unwrap();
        let manual_transaction = build_asset_issue_transaction(input());

        assert_eq!(
            adapter_transaction.to_fc_bytes().unwrap(),
            manual_transaction.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn renders_signed_asset_issue_json_for_broadcast() {
        let mut transaction = build_asset_issue_transaction(input());
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
                "operations": [[14, {
                    "fee": { "amount": 200000, "asset_id": "1.3.0" },
                    "issuer": "1.2.100",
                    "asset_to_issue": { "amount": 100000, "asset_id": "1.3.1" },
                    "issue_to_account": "1.2.101",
                    "memo": null,
                    "extensions": []
                }]],
                "extensions": [],
                "signatures": ["1f".repeat(65)]
            })
        );
    }

    #[test]
    fn rejects_non_asset_issue_operations() {
        let signed = SignedTransaction {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            operations: vec![Operation::TransferOperation(Box::new(TransferOperation {
                fee: Asset {
                    amount: 200_000,
                    asset_id: AssetId("1.3.0".to_string()),
                },
                from: AccountId("1.2.100".to_string()),
                to: AccountId("1.2.101".to_string()),
                amount: Asset {
                    amount: 100_000,
                    asset_id: AssetId("1.3.0".to_string()),
                },
                memo: None,
                extensions: FutureExtensions::empty(),
            }))],
            extensions: FutureExtensions::empty(),
            signatures: vec![],
        };

        assert_eq!(
            signed_transaction_json(&signed).unwrap_err(),
            AssetIssueJsonError::UnsupportedOperation
        );
    }

    #[test]
    fn rejects_asset_issue_memo_json() {
        let mut transaction = build_asset_issue_transaction(input());
        let Operation::AssetIssueOperation(operation) = &mut transaction.operations[0] else {
            panic!("expected asset_issue operation");
        };
        operation.memo = Some(MemoData {
            from: None,
            amount: Asset {
                amount: 1,
                asset_id: AssetId("1.3.1".to_string()),
            },
            blinding_factor: vec![],
            commitment: vec![],
            check: 0,
        });
        let signed = SignedTransaction {
            ref_block_num: transaction.ref_block_num,
            ref_block_prefix: transaction.ref_block_prefix,
            expiration: transaction.expiration,
            operations: transaction.operations,
            extensions: transaction.extensions,
            signatures: vec![],
        };

        assert_eq!(
            signed_transaction_json(&signed).unwrap_err(),
            AssetIssueJsonError::UnsupportedMemo
        );
    }
}
