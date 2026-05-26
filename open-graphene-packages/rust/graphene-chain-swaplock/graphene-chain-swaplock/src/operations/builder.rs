use open_graphene_sdk_core::TransactionHeader;
use open_graphene_sdk_operations::GrapheneOperationBuilderTypes;

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, LimitOrderId};
use graphene_chain_swaplock_bindings::generated::static_variants::{FutureExtensions, Operation};
use graphene_chain_swaplock_bindings::generated::types::{Asset, Transaction};

pub struct SwaplockOperationBuilderTypes;

impl GrapheneOperationBuilderTypes for SwaplockOperationBuilderTypes {
    type Transaction = Transaction;
    type Operation = Operation;
    type Asset = Asset;
    type AccountId = AccountId;
    type AssetId = AssetId;
    type LimitOrderId = LimitOrderId;
    type FutureExtensions = Vec<FutureExtensions>;

    fn asset(amount: i64, asset_id: Self::AssetId) -> Self::Asset {
        Asset::new(amount, asset_id)
    }

    fn empty_extensions() -> Self::FutureExtensions {
        vec![]
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
