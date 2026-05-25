use open_graphene_sdk_core::TransactionHeader;

use crate::builder::GrapheneOperationBuilderTypes;
use crate::common::{AccountRefInput, AssetAmountInput, FeeInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferInput {
    pub header: TransactionHeader,
    pub from: AccountRefInput,
    pub to: AccountRefInput,
    pub amount: AssetAmountInput,
    pub fee: FeeInput,
}

impl TransferInput {
    pub fn new(
        header: TransactionHeader,
        from_id: impl Into<String>,
        to_id: impl Into<String>,
        amount: i64,
        asset_id: impl Into<String>,
        fee_amount: i64,
        fee_asset_id: impl Into<String>,
    ) -> Self {
        Self {
            header,
            from: AccountRefInput::new(from_id),
            to: AccountRefInput::new(to_id),
            amount: AssetAmountInput::new(amount, asset_id),
            fee: FeeInput::new(fee_amount, fee_asset_id),
        }
    }
}

pub trait TransferAdapter {
    type Transaction;
    type Error;

    fn build_transfer_transaction(input: TransferInput) -> Result<Self::Transaction, Self::Error>;
}

pub trait TransferChainTypes: GrapheneOperationBuilderTypes {
    type TransferOperation;

    fn transfer_operation_without_memo(
        fee: Self::Asset,
        from: Self::AccountId,
        to: Self::AccountId,
        amount: Self::Asset,
        extensions: Self::FutureExtensions,
    ) -> Self::TransferOperation;

    fn operation_transfer(operation: Self::TransferOperation) -> Self::Operation;
}

pub fn build_transfer_transaction_for<C: TransferChainTypes>(
    input: TransferInput,
) -> C::Transaction {
    let fee = C::asset(input.fee.amount, input.fee.asset_id.into());
    let amount = C::asset(input.amount.amount, input.amount.asset_id.into());
    let operation = C::transfer_operation_without_memo(
        fee,
        input.from.id.into(),
        input.to.id.into(),
        amount,
        C::empty_extensions(),
    );

    C::transaction(
        input.header,
        vec![C::operation_transfer(operation)],
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
    fn transfer_input_constructor_wraps_raw_fields() {
        let input = TransferInput::new(
            header(),
            "1.2.100",
            "1.2.101",
            100_000,
            "1.3.1",
            200_000,
            "1.3.0",
        );

        assert_eq!(input.header.ref_block_num, 2);
        assert_eq!(input.from.id, "1.2.100");
        assert_eq!(input.to.id, "1.2.101");
        assert_eq!(input.amount, AssetAmountInput::new(100_000, "1.3.1"));
        assert_eq!(input.fee, FeeInput::new(200_000, "1.3.0"));
    }
}
