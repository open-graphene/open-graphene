use open_graphene_sdk_core::TransactionHeader;

use crate::common::{AccountRefInput, FeeInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub issuer: AccountRefInput,
    pub symbol: String,
    pub precision: u8,
    pub max_supply: i64,
    pub description: String,
}

impl AssetCreateInput {
    pub fn uia(
        header: TransactionHeader,
        fee: FeeInput,
        issuer_id: impl Into<String>,
        symbol: impl Into<String>,
        precision: u8,
        max_supply: i64,
        description: impl Into<String>,
    ) -> Self {
        Self {
            header,
            fee,
            issuer: AccountRefInput::new(issuer_id),
            symbol: symbol.into(),
            precision,
            max_supply,
            description: description.into(),
        }
    }
}

pub trait AssetCreateAdapter {
    type Transaction;
    type Error;

    fn build_asset_create_transaction(
        input: AssetCreateInput,
    ) -> Result<Self::Transaction, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> TransactionHeader {
        TransactionHeader {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
        }
    }

    #[test]
    fn asset_create_uia_constructor_wraps_minimal_asset_create_fields() {
        let input = AssetCreateInput::uia(
            header(),
            FeeInput::new(500_000, "1.3.0"),
            "1.2.100",
            "OGT12345",
            5,
            1_000_000_000_000,
            "open-graphene live asset_create proof",
        );

        assert_eq!(input.issuer.id, "1.2.100");
        assert_eq!(input.symbol, "OGT12345");
        assert_eq!(input.precision, 5);
        assert_eq!(input.max_supply, 1_000_000_000_000);
        assert_eq!(input.description, "open-graphene live asset_create proof");
    }
}
