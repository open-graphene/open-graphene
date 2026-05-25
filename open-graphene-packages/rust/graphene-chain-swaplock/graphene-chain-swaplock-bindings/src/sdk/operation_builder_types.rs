use open_graphene_sdk_core::TransactionHeader;
use open_graphene_sdk_operations::GrapheneOperationBuilderTypes;

use crate::generated::ids::{AccountId, AssetId};
use crate::generated::static_variants::{FutureExtensions, Operation};
use crate::generated::types::{Asset, Transaction};

pub struct SwaplockOperationBuilderTypes;

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

impl GrapheneOperationBuilderTypes for SwaplockOperationBuilderTypes {
    type Transaction = Transaction;
    type Operation = Operation;
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
