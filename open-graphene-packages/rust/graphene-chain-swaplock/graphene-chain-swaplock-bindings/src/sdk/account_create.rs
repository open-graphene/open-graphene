use std::convert::Infallible;

use open_graphene_sdk_operations::{
    AccountCreateAdapter, AccountCreateChainTypes, AccountCreateInput,
    build_account_create_transaction_for,
};
use serde_json::{Value, json};
use thiserror::Error;

use crate::generated::ids::{AccountId, AssetId};
use crate::generated::operations::AccountCreateOperation;
use crate::generated::static_variants::{FutureExtensions, Operation};
use crate::generated::types::{
    AccountCreateOperationExt, AccountOptions, Asset, Authority, SignedTransaction, Transaction,
};
use crate::sdk::operation_builder_types::SwaplockOperationBuilderTypes;

#[derive(Clone, Debug, PartialEq)]
pub struct AccountCreateTransactionInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    pub fee_amount: i64,
    pub fee_asset_id: String,
    pub registrar_id: String,
    pub referrer_id: String,
    pub referrer_percent: u16,
    pub name: String,
    pub owner: Authority,
    pub active: Authority,
    pub options: AccountOptions,
    pub extensions: Option<AccountCreateOperationExt>,
}

impl AccountCreateChainTypes for SwaplockOperationBuilderTypes {
    type Authority = Authority;
    type AccountOptions = AccountOptions;
    type AccountCreateOperation = AccountCreateOperation;
    type AccountCreateOperationExtensions = AccountCreateOperationExt;

    fn single_key_authority(public_key: String) -> Self::Authority {
        crate::sdk::account_create::single_key_authority(public_key)
    }

    fn account_options(memo_key: String, voting_account: Self::AccountId) -> Self::AccountOptions {
        crate::sdk::account_create::account_options(memo_key, voting_account.0)
    }

    fn empty_account_create_extensions() -> Self::AccountCreateOperationExtensions {
        crate::sdk::account_create::empty_account_create_extensions()
    }

    fn account_create_operation(
        fee: Self::Asset,
        registrar: Self::AccountId,
        referrer: Self::AccountId,
        referrer_percent: u16,
        name: String,
        owner: Self::Authority,
        active: Self::Authority,
        options: Self::AccountOptions,
        extensions: Self::AccountCreateOperationExtensions,
    ) -> Self::AccountCreateOperation {
        AccountCreateOperation {
            fee,
            registrar,
            referrer,
            referrer_percent,
            name,
            owner,
            active,
            options,
            extensions,
        }
    }

    fn operation_account_create(operation: Self::AccountCreateOperation) -> Self::Operation {
        Operation::account_create(operation)
    }
}

pub struct SwaplockAccountCreateAdapter;

impl SwaplockAccountCreateAdapter {
    pub fn build_account_create_transaction(
        input: AccountCreateInput,
    ) -> Result<Transaction, Infallible> {
        <Self as AccountCreateAdapter>::build_account_create_transaction(input)
    }
}

impl AccountCreateAdapter for SwaplockAccountCreateAdapter {
    type Transaction = Transaction;
    type Error = Infallible;

    fn build_account_create_transaction(
        input: AccountCreateInput,
    ) -> Result<Self::Transaction, Self::Error> {
        Ok(build_account_create_transaction_for::<
            SwaplockOperationBuilderTypes,
        >(input))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AccountCreateJsonError {
    #[error("signed account-create JSON can only render account_create operations")]
    UnsupportedOperation,
    #[error("account-create broadcast JSON only supports empty generated extension fields")]
    UnsupportedExtensions,
}

pub fn build_account_create_transaction(input: AccountCreateTransactionInput) -> Transaction {
    Transaction {
        ref_block_num: input.ref_block_num,
        ref_block_prefix: input.ref_block_prefix,
        expiration: input.expiration,
        operations: vec![Operation::AccountCreateOperation(Box::new(
            AccountCreateOperation {
                fee: Asset::new(input.fee_amount, AssetId::new(input.fee_asset_id)),
                registrar: AccountId(input.registrar_id),
                referrer: AccountId(input.referrer_id),
                referrer_percent: input.referrer_percent,
                name: input.name,
                owner: input.owner,
                active: input.active,
                options: input.options,
                extensions: input
                    .extensions
                    .unwrap_or_else(empty_account_create_extensions),
            },
        ))],
        extensions: FutureExtensions::empty(),
    }
}

pub fn single_key_authority(public_key: String) -> Authority {
    Authority {
        weight_threshold: 1,
        account_auths: Vec::new(),
        key_auths: vec![(public_key, 1)],
        address_auths: Vec::new(),
    }
}

pub fn account_options(memo_key: String, voting_account_id: String) -> AccountOptions {
    AccountOptions {
        memo_key,
        voting_account: AccountId(voting_account_id),
        num_witness: 0,
        num_committee: 0,
        votes: Vec::new(),
        extensions: FutureExtensions::empty(),
    }
}

pub fn empty_account_create_extensions() -> AccountCreateOperationExt {
    AccountCreateOperationExt {
        null_ext: None,
        owner_special_authority: None,
        active_special_authority: None,
    }
}

pub fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<Value, AccountCreateJsonError> {
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

fn operation_json(operation: &Operation) -> Result<Value, AccountCreateJsonError> {
    let operation = operation
        .as_account_create()
        .ok_or(AccountCreateJsonError::UnsupportedOperation)?;

    Ok(json!([
        5,
        {
            "fee": asset_json(&operation.fee),
            "registrar": operation.registrar.0,
            "referrer": operation.referrer.0,
            "referrer_percent": operation.referrer_percent,
            "name": operation.name,
            "owner": authority_json(&operation.owner),
            "active": authority_json(&operation.active),
            "options": account_options_json(&operation.options)?,
            "extensions": account_create_extensions_json(&operation.extensions)?,
        }
    ]))
}

fn authority_json(authority: &Authority) -> Value {
    json!({
        "weight_threshold": authority.weight_threshold,
        "account_auths": authority
            .account_auths
            .iter()
            .map(|(account, weight)| json!([account.0, weight]))
            .collect::<Vec<_>>(),
        "key_auths": authority
            .key_auths
            .iter()
            .map(|(key, weight)| json!([key, weight]))
            .collect::<Vec<_>>(),
        "address_auths": authority
            .address_auths
            .iter()
            .map(|(address, weight)| json!([address, weight]))
            .collect::<Vec<_>>(),
    })
}

fn account_options_json(options: &AccountOptions) -> Result<Value, AccountCreateJsonError> {
    if !matches!(options.extensions, FutureExtensions::VoidT(_)) {
        return Err(AccountCreateJsonError::UnsupportedExtensions);
    }
    Ok(json!({
        "memo_key": options.memo_key,
        "voting_account": options.voting_account.0,
        "num_witness": options.num_witness,
        "num_committee": options.num_committee,
        "votes": options.votes.iter().map(|vote| vote.0.clone()).collect::<Vec<_>>(),
        "extensions": [],
    }))
}

fn account_create_extensions_json(
    extensions: &AccountCreateOperationExt,
) -> Result<Value, AccountCreateJsonError> {
    if extensions.null_ext.is_none()
        && extensions.owner_special_authority.is_none()
        && extensions.active_special_authority.is_none()
    {
        return Ok(json!([]));
    }
    Err(AccountCreateJsonError::UnsupportedExtensions)
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
    use crate::generated::ids::VoteId;
    use crate::generated::operations::TransferOperation;
    use crate::generated::static_variants::SpecialAuthority;
    use crate::generated::types::{Signature, TopHoldersSpecialAuthority};

    const PUBLIC_KEY: &str = "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV";

    fn input() -> AccountCreateTransactionInput {
        AccountCreateTransactionInput {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            fee_amount: 500_000,
            fee_asset_id: "1.3.0".to_string(),
            registrar_id: "1.2.100".to_string(),
            referrer_id: "1.2.101".to_string(),
            referrer_percent: 5_000,
            name: "new-account".to_string(),
            owner: single_key_authority(PUBLIC_KEY.to_string()),
            active: single_key_authority(PUBLIC_KEY.to_string()),
            options: account_options(PUBLIC_KEY.to_string(), "1.2.5".to_string()),
            extensions: None,
        }
    }

    fn expected_transaction(input: AccountCreateTransactionInput) -> Transaction {
        Transaction {
            ref_block_num: input.ref_block_num,
            ref_block_prefix: input.ref_block_prefix,
            expiration: input.expiration,
            operations: vec![Operation::AccountCreateOperation(Box::new(
                AccountCreateOperation {
                    fee: Asset::new(input.fee_amount, AssetId::new(input.fee_asset_id)),
                    registrar: AccountId(input.registrar_id),
                    referrer: AccountId(input.referrer_id),
                    referrer_percent: input.referrer_percent,
                    name: input.name,
                    owner: input.owner,
                    active: input.active,
                    options: input.options,
                    extensions: input
                        .extensions
                        .unwrap_or_else(empty_account_create_extensions),
                },
            ))],
            extensions: FutureExtensions::empty(),
        }
    }

    #[test]
    fn builds_account_create_transaction_with_generated_swaplock_types() {
        let input = input();
        let transaction = build_account_create_transaction(input.clone());
        let expected = expected_transaction(input);

        assert_eq!(
            transaction.to_fc_bytes().unwrap(),
            expected.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn common_account_create_adapter_matches_manual_swaplock_builder() {
        let adapter_transaction =
            SwaplockAccountCreateAdapter::build_account_create_transaction(AccountCreateInput {
                header: open_graphene_sdk_core::TransactionHeader {
                    ref_block_num: 2,
                    ref_block_prefix: 3,
                    expiration: "2026-05-25T12:01:00".to_string(),
                },
                fee: open_graphene_sdk_operations::FeeInput::new(500_000, "1.3.0"),
                registrar: open_graphene_sdk_operations::AccountRefInput::new("1.2.100"),
                referrer: open_graphene_sdk_operations::AccountRefInput::new("1.2.101"),
                referrer_percent: 5_000,
                name: "new-account".to_string(),
                owner: open_graphene_sdk_operations::SingleKeyAuthorityInput::new(PUBLIC_KEY),
                active: open_graphene_sdk_operations::SingleKeyAuthorityInput::new(PUBLIC_KEY),
                memo_key: open_graphene_sdk_operations::PublicKeyInput::new(PUBLIC_KEY),
                voting_account: open_graphene_sdk_operations::AccountRefInput::new("1.2.5"),
            })
            .unwrap();
        let manual_transaction = build_account_create_transaction(input());

        assert_eq!(
            adapter_transaction.to_fc_bytes().unwrap(),
            manual_transaction.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn rejects_account_create_transaction_with_special_authority_extension() {
        let mut input = input();
        input.options.votes = vec![VoteId("1:5".to_string())];
        input.extensions = Some(AccountCreateOperationExt {
            null_ext: None,
            owner_special_authority: Some(SpecialAuthority::TopHoldersSpecialAuthority(Box::new(
                TopHoldersSpecialAuthority {
                    asset: AssetId("1.3.0".to_string()),
                    num_top_holders: 3,
                },
            ))),
            active_special_authority: None,
        });

        let transaction = build_account_create_transaction(input);

        assert!(transaction.to_fc_bytes().is_err());
    }

    #[test]
    fn renders_signed_account_create_json_for_broadcast() {
        let mut transaction = build_account_create_transaction(input());
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
                "operations": [[5, {
                    "fee": { "amount": 500000, "asset_id": "1.3.0" },
                    "registrar": "1.2.100",
                    "referrer": "1.2.101",
                    "referrer_percent": 5000,
                    "name": "new-account",
                    "owner": {
                        "weight_threshold": 1,
                        "account_auths": [],
                        "key_auths": [[PUBLIC_KEY, 1]],
                        "address_auths": []
                    },
                    "active": {
                        "weight_threshold": 1,
                        "account_auths": [],
                        "key_auths": [[PUBLIC_KEY, 1]],
                        "address_auths": []
                    },
                    "options": {
                        "memo_key": PUBLIC_KEY,
                        "voting_account": "1.2.5",
                        "num_witness": 0,
                        "num_committee": 0,
                        "votes": [],
                        "extensions": []
                    },
                    "extensions": []
                }]],
                "extensions": [],
                "signatures": ["1f".repeat(65)]
            })
        );
    }

    #[test]
    fn rejects_non_account_create_operations() {
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
                to: AccountId("1.2.0".to_string()),
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
            AccountCreateJsonError::UnsupportedOperation
        );
    }
}
