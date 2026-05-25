use std::convert::Infallible;

use open_graphene_sdk_core::TransactionHeader;
use open_graphene_sdk_operations::{
    build_asset_create_transaction_for, signed_transaction_broadcast_json, AssetCreateAdapter,
    AssetCreateChainTypes, AssetCreateInput, SignedTransactionJsonParts,
};
use serde_json::{json, Value};
use thiserror::Error;

use crate::operation_builder_types::SwaplockOperationBuilderTypes;
use graphene_chain_swaplock_bindings::generated::ids::AssetId;
use graphene_chain_swaplock_bindings::generated::operations::AssetCreateOperation;
use graphene_chain_swaplock_bindings::generated::static_variants::{FutureExtensions, Operation};
use graphene_chain_swaplock_bindings::generated::types::{
    AdditionalAssetOptions, Asset, AssetOptions, Price, SignedTransaction, Transaction,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetCreateTransactionInput {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
    pub fee_amount: i64,
    pub fee_asset_id: String,
    pub issuer_id: String,
    pub symbol: String,
    pub precision: u8,
    pub max_supply: i64,
    pub description: String,
}

impl AssetCreateChainTypes for SwaplockOperationBuilderTypes {
    type Price = Price;
    type AdditionalAssetOptions = AdditionalAssetOptions;
    type AssetOptions = AssetOptions;
    type AssetCreateOperation = AssetCreateOperation;

    fn price(base: Self::Asset, quote: Self::Asset) -> Self::Price {
        Price::new(base, quote)
    }

    fn empty_asset_create_additional_options() -> Self::AdditionalAssetOptions {
        crate::asset_create::empty_additional_asset_options()
    }

    fn asset_options(
        max_supply: i64,
        market_fee_percent: u16,
        max_market_fee: i64,
        issuer_permissions: u16,
        flags: u16,
        core_exchange_rate: Self::Price,
        description: String,
        extensions: Self::AdditionalAssetOptions,
    ) -> Self::AssetOptions {
        AssetOptions {
            max_supply,
            market_fee_percent,
            max_market_fee,
            issuer_permissions,
            flags,
            core_exchange_rate,
            whitelist_authorities: Vec::new(),
            blacklist_authorities: Vec::new(),
            whitelist_markets: Vec::new(),
            blacklist_markets: Vec::new(),
            description,
            extensions,
        }
    }

    fn user_issued_asset_create_operation(
        fee: Self::Asset,
        issuer: Self::AccountId,
        symbol: String,
        precision: u8,
        common_options: Self::AssetOptions,
        extensions: Self::FutureExtensions,
    ) -> Self::AssetCreateOperation {
        AssetCreateOperation {
            fee,
            issuer,
            symbol,
            precision,
            common_options,
            bitasset_opts: None,
            is_prediction_market: false,
            extensions,
        }
    }

    fn operation_asset_create(operation: Self::AssetCreateOperation) -> Self::Operation {
        Operation::asset_create(operation)
    }
}

pub struct SwaplockAssetCreateAdapter;

impl SwaplockAssetCreateAdapter {
    pub fn build_asset_create_transaction(
        input: AssetCreateInput,
    ) -> Result<Transaction, Infallible> {
        <Self as AssetCreateAdapter>::build_asset_create_transaction(input)
    }
}

impl AssetCreateAdapter for SwaplockAssetCreateAdapter {
    type Transaction = Transaction;
    type Error = Infallible;

    fn build_asset_create_transaction(
        input: AssetCreateInput,
    ) -> Result<Self::Transaction, Self::Error> {
        Ok(build_asset_create_transaction_for::<
            SwaplockOperationBuilderTypes,
        >(input))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssetCreateJsonError {
    #[error("signed asset-create JSON can only render asset_create operations")]
    UnsupportedOperation,
    #[error(
        "asset_create broadcast JSON only supports user-issued assets without bitasset options"
    )]
    UnsupportedBitasset,
    #[error("asset_create broadcast JSON does not support prediction markets yet")]
    UnsupportedPredictionMarket,
    #[error("asset_create broadcast JSON only supports empty authority and market lists")]
    UnsupportedLists,
    #[error("asset_create broadcast JSON only supports empty generated extension fields")]
    UnsupportedExtensions,
}

pub fn build_asset_create_transaction(input: AssetCreateTransactionInput) -> Transaction {
    build_asset_create_transaction_for::<SwaplockOperationBuilderTypes>(AssetCreateInput::uia(
        TransactionHeader {
            ref_block_num: input.ref_block_num,
            ref_block_prefix: input.ref_block_prefix,
            expiration: input.expiration,
        },
        open_graphene_sdk_operations::FeeInput::new(input.fee_amount, input.fee_asset_id),
        input.issuer_id,
        input.symbol,
        input.precision,
        input.max_supply,
        input.description,
    ))
}

pub fn minimal_asset_options(max_supply: i64, description: String) -> AssetOptions {
    AssetOptions {
        max_supply,
        market_fee_percent: 0,
        max_market_fee: 0,
        issuer_permissions: 0,
        flags: 0,
        core_exchange_rate: asset_create_core_exchange_rate(),
        whitelist_authorities: Vec::new(),
        blacklist_authorities: Vec::new(),
        whitelist_markets: Vec::new(),
        blacklist_markets: Vec::new(),
        description,
        extensions: empty_additional_asset_options(),
    }
}

pub fn asset_create_core_exchange_rate() -> Price {
    Price::new(
        Asset::new(1, AssetId::new("1.3.0")),
        Asset::new(1, AssetId::new("1.3.1")),
    )
}

pub fn empty_additional_asset_options() -> AdditionalAssetOptions {
    AdditionalAssetOptions {
        reward_percent: None,
        whitelist_market_fee_sharing: None,
        taker_fee_percent: None,
    }
}

pub fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<Value, AssetCreateJsonError> {
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

fn operation_json(operation: &Operation) -> Result<Value, AssetCreateJsonError> {
    let operation = operation
        .as_asset_create()
        .ok_or(AssetCreateJsonError::UnsupportedOperation)?;

    if operation.bitasset_opts.is_some() {
        return Err(AssetCreateJsonError::UnsupportedBitasset);
    }
    if operation.is_prediction_market {
        return Err(AssetCreateJsonError::UnsupportedPredictionMarket);
    }
    if !matches!(operation.extensions, FutureExtensions::VoidT(_)) {
        return Err(AssetCreateJsonError::UnsupportedExtensions);
    }

    Ok(json!([
        10,
        {
            "fee": asset_json(&operation.fee),
            "issuer": operation.issuer.0,
            "symbol": operation.symbol,
            "precision": operation.precision,
            "common_options": asset_options_json(&operation.common_options)?,
            "bitasset_opts": null,
            "is_prediction_market": false,
            "extensions": []
        }
    ]))
}

fn asset_options_json(options: &AssetOptions) -> Result<Value, AssetCreateJsonError> {
    if !options.whitelist_authorities.is_empty()
        || !options.blacklist_authorities.is_empty()
        || !options.whitelist_markets.is_empty()
        || !options.blacklist_markets.is_empty()
    {
        return Err(AssetCreateJsonError::UnsupportedLists);
    }
    if options.extensions.reward_percent.is_some()
        || options.extensions.whitelist_market_fee_sharing.is_some()
        || options.extensions.taker_fee_percent.is_some()
    {
        return Err(AssetCreateJsonError::UnsupportedExtensions);
    }

    Ok(json!({
        "max_supply": options.max_supply,
        "market_fee_percent": options.market_fee_percent,
        "max_market_fee": options.max_market_fee,
        "issuer_permissions": options.issuer_permissions,
        "flags": options.flags,
        "core_exchange_rate": price_json(&options.core_exchange_rate),
        "whitelist_authorities": [],
        "blacklist_authorities": [],
        "whitelist_markets": [],
        "blacklist_markets": [],
        "description": options.description,
        "extensions": [],
    }))
}

fn price_json(price: &Price) -> Value {
    json!({
        "base": asset_json(&price.base),
        "quote": asset_json(&price.quote),
    })
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
    use graphene_chain_swaplock_bindings::generated::ids::AccountId;
    use graphene_chain_swaplock_bindings::generated::operations::TransferOperation;
    use graphene_chain_swaplock_bindings::generated::types::{
        BitassetOptions, BitassetOptionsExt, Signature,
    };
    use graphene_chain_swaplock_bindings::generated::FcSerialize;

    fn input() -> AssetCreateTransactionInput {
        AssetCreateTransactionInput {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
            fee_amount: 500_000,
            fee_asset_id: "1.3.0".to_string(),
            issuer_id: "1.2.100".to_string(),
            symbol: "OGT12345".to_string(),
            precision: 5,
            max_supply: 1_000_000_000_000,
            description: "open-graphene live asset_create proof".to_string(),
        }
    }

    fn expected_transaction(input: AssetCreateTransactionInput) -> Transaction {
        Transaction {
            ref_block_num: input.ref_block_num,
            ref_block_prefix: input.ref_block_prefix,
            expiration: input.expiration,
            operations: vec![Operation::AssetCreateOperation(Box::new(
                AssetCreateOperation {
                    fee: Asset::new(input.fee_amount, AssetId::new(input.fee_asset_id)),
                    issuer: AccountId(input.issuer_id),
                    symbol: input.symbol,
                    precision: input.precision,
                    common_options: minimal_asset_options(input.max_supply, input.description),
                    bitasset_opts: None,
                    is_prediction_market: false,
                    extensions: FutureExtensions::empty(),
                },
            ))],
            extensions: FutureExtensions::empty(),
        }
    }

    #[test]
    fn builds_asset_create_transaction_with_generated_swaplock_types() {
        let input = input();
        let transaction = build_asset_create_transaction(input.clone());
        let expected = expected_transaction(input);

        assert_eq!(
            transaction.to_fc_bytes().unwrap(),
            expected.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn common_asset_create_adapter_matches_manual_swaplock_builder() {
        let adapter_transaction =
            SwaplockAssetCreateAdapter::build_asset_create_transaction(AssetCreateInput {
                header: open_graphene_sdk_core::TransactionHeader {
                    ref_block_num: 2,
                    ref_block_prefix: 3,
                    expiration: "2026-05-25T12:01:00".to_string(),
                },
                fee: open_graphene_sdk_operations::FeeInput::new(500_000, "1.3.0"),
                issuer: open_graphene_sdk_operations::AccountRefInput::new("1.2.100"),
                symbol: "OGT12345".to_string(),
                precision: 5,
                max_supply: 1_000_000_000_000,
                description: "open-graphene live asset_create proof".to_string(),
            })
            .unwrap();
        let manual_transaction = build_asset_create_transaction(input());

        assert_eq!(
            adapter_transaction.to_fc_bytes().unwrap(),
            manual_transaction.to_fc_bytes().unwrap()
        );
    }

    #[test]
    fn renders_signed_asset_create_json_for_broadcast() {
        let mut transaction = build_asset_create_transaction(input());
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
                "operations": [[10, {
                    "fee": { "amount": 500000, "asset_id": "1.3.0" },
                    "issuer": "1.2.100",
                    "symbol": "OGT12345",
                    "precision": 5,
                    "common_options": {
                        "max_supply": 1000000000000i64,
                        "market_fee_percent": 0,
                        "max_market_fee": 0,
                        "issuer_permissions": 0,
                        "flags": 0,
                        "core_exchange_rate": {
                            "base": { "amount": 1, "asset_id": "1.3.0" },
                            "quote": { "amount": 1, "asset_id": "1.3.1" }
                        },
                        "whitelist_authorities": [],
                        "blacklist_authorities": [],
                        "whitelist_markets": [],
                        "blacklist_markets": [],
                        "description": "open-graphene live asset_create proof",
                        "extensions": []
                    },
                    "bitasset_opts": null,
                    "is_prediction_market": false,
                    "extensions": []
                }]],
                "extensions": [],
                "signatures": ["1f".repeat(65)]
            })
        );
    }

    #[test]
    fn rejects_non_asset_create_operations() {
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
            AssetCreateJsonError::UnsupportedOperation
        );
    }

    #[test]
    fn rejects_asset_create_bitasset_options() {
        let mut transaction = build_asset_create_transaction(input());
        let Operation::AssetCreateOperation(operation) = &mut transaction.operations[0] else {
            panic!("expected asset_create operation");
        };
        operation.bitasset_opts = Some(BitassetOptions {
            feed_lifetime_sec: 86_400,
            minimum_feeds: 1,
            force_settlement_delay_sec: 86_400,
            force_settlement_offset_percent: 0,
            maximum_force_settlement_volume: 0,
            short_backing_asset: AssetId("1.3.0".to_string()),
            extensions: BitassetOptionsExt {
                initial_collateral_ratio: None,
            },
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
            AssetCreateJsonError::UnsupportedBitasset
        );
    }

    #[test]
    fn rejects_additional_asset_options() {
        let mut transaction = build_asset_create_transaction(input());
        let Operation::AssetCreateOperation(operation) = &mut transaction.operations[0] else {
            panic!("expected asset_create operation");
        };
        operation.common_options.extensions.reward_percent = Some(1);
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
            AssetCreateJsonError::UnsupportedExtensions
        );
    }
}
