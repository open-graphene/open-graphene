use open_graphene_sdk_core::TransactionHeader;

use crate::builder::GrapheneOperationBuilderTypes;
use crate::common::{AccountRefInput, FeeInput, LimitOrderRefInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOrderCancelInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub fee_paying_account: AccountRefInput,
    pub order: LimitOrderRefInput,
}

impl LimitOrderCancelInput {
    pub fn new(
        header: TransactionHeader,
        fee: FeeInput,
        fee_paying_account_id: impl Into<String>,
        order_id: impl Into<String>,
    ) -> Self {
        Self {
            header,
            fee,
            fee_paying_account: AccountRefInput::new(fee_paying_account_id),
            order: LimitOrderRefInput::new(order_id),
        }
    }
}

pub trait LimitOrderCancelAdapter {
    type Transaction;
    type Error;

    fn build_limit_order_cancel_transaction(
        input: LimitOrderCancelInput,
    ) -> Result<Self::Transaction, Self::Error>;
}

pub trait LimitOrderCancelChainTypes: GrapheneOperationBuilderTypes {
    type LimitOrderCancelOperation;

    fn limit_order_cancel_operation(
        fee: Self::Asset,
        fee_paying_account: Self::AccountId,
        order: Self::LimitOrderId,
        extensions: Self::FutureExtensions,
    ) -> Self::LimitOrderCancelOperation;

    fn operation_limit_order_cancel(operation: Self::LimitOrderCancelOperation) -> Self::Operation;
}

pub fn build_limit_order_cancel_transaction_for<C: LimitOrderCancelChainTypes>(
    input: LimitOrderCancelInput,
) -> C::Transaction {
    let fee = C::asset(input.fee.amount, input.fee.asset_id.into());
    let operation = C::limit_order_cancel_operation(
        fee,
        input.fee_paying_account.id.into(),
        input.order.id.into(),
        C::empty_extensions(),
    );

    C::transaction(
        input.header,
        vec![C::operation_limit_order_cancel(operation)],
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
    fn limit_order_cancel_constructor_wraps_minimal_fields() {
        let input = LimitOrderCancelInput::new(
            header(),
            FeeInput::new(200_000, "1.3.0"),
            "1.2.100",
            "1.7.123",
        );

        assert_eq!(input.fee, FeeInput::new(200_000, "1.3.0"));
        assert_eq!(input.fee_paying_account, AccountRefInput::new("1.2.100"));
        assert_eq!(input.order, LimitOrderRefInput::new("1.7.123"));
    }
}
