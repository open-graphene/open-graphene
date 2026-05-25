use open_graphene_sdk_core::TransactionHeader;

use crate::builder::GrapheneOperationBuilderTypes;
use crate::common::{AccountRefInput, AssetAmountInput, FeeInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOrderCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub seller: AccountRefInput,
    pub amount_to_sell: AssetAmountInput,
    pub min_to_receive: AssetAmountInput,
    pub expiration: String,
    pub fill_or_kill: bool,
}

impl LimitOrderCreateInput {
    pub fn new(
        header: TransactionHeader,
        fee: FeeInput,
        seller_id: impl Into<String>,
        amount_to_sell: AssetAmountInput,
        min_to_receive: AssetAmountInput,
        expiration: impl Into<String>,
        fill_or_kill: bool,
    ) -> Self {
        Self {
            header,
            fee,
            seller: AccountRefInput::new(seller_id),
            amount_to_sell,
            min_to_receive,
            expiration: expiration.into(),
            fill_or_kill,
        }
    }
}

pub trait LimitOrderCreateAdapter {
    type Transaction;
    type Error;

    fn build_limit_order_create_transaction(
        input: LimitOrderCreateInput,
    ) -> Result<Self::Transaction, Self::Error>;
}

pub trait LimitOrderCreateChainTypes: GrapheneOperationBuilderTypes {
    type LimitOrderCreateOperation;

    fn limit_order_create_operation(
        fee: Self::Asset,
        seller: Self::AccountId,
        amount_to_sell: Self::Asset,
        min_to_receive: Self::Asset,
        expiration: String,
        fill_or_kill: bool,
        extensions: Self::FutureExtensions,
    ) -> Self::LimitOrderCreateOperation;

    fn operation_limit_order_create(operation: Self::LimitOrderCreateOperation) -> Self::Operation;
}

pub fn build_limit_order_create_transaction_for<C: LimitOrderCreateChainTypes>(
    input: LimitOrderCreateInput,
) -> C::Transaction {
    let fee = C::asset(input.fee.amount, input.fee.asset_id.into());
    let amount_to_sell = C::asset(
        input.amount_to_sell.amount,
        input.amount_to_sell.asset_id.into(),
    );
    let min_to_receive = C::asset(
        input.min_to_receive.amount,
        input.min_to_receive.asset_id.into(),
    );
    let operation = C::limit_order_create_operation(
        fee,
        input.seller.id.into(),
        amount_to_sell,
        min_to_receive,
        input.expiration,
        input.fill_or_kill,
        C::empty_extensions(),
    );

    C::transaction(
        input.header,
        vec![C::operation_limit_order_create(operation)],
        C::empty_extensions(),
    )
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
    fn limit_order_create_constructor_wraps_raw_order_fields() {
        let input = LimitOrderCreateInput::new(
            header(),
            FeeInput::new(200_000, "1.3.0"),
            "1.2.100",
            AssetAmountInput::new(100_000, "1.3.1"),
            AssetAmountInput::new(250_000, "1.3.0"),
            "2026-05-26T12:01:00",
            false,
        );

        assert_eq!(input.fee, FeeInput::new(200_000, "1.3.0"));
        assert_eq!(input.seller, AccountRefInput::new("1.2.100"));
        assert_eq!(
            input.amount_to_sell,
            AssetAmountInput::new(100_000, "1.3.1")
        );
        assert_eq!(
            input.min_to_receive,
            AssetAmountInput::new(250_000, "1.3.0")
        );
        assert_eq!(input.expiration, "2026-05-26T12:01:00");
        assert!(!input.fill_or_kill);
    }
}
