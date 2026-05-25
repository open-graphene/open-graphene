use open_graphene_sdk_core::TransactionHeader;

use crate::builder::GrapheneOperationBuilderTypes;
use crate::common::{AccountRefInput, AssetAmountInput, FeeInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIssueInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub issuer: AccountRefInput,
    pub issue_to_account: AccountRefInput,
    pub asset_to_issue: AssetAmountInput,
}

impl AssetIssueInput {
    pub fn new(
        header: TransactionHeader,
        fee: FeeInput,
        issuer_id: impl Into<String>,
        issue_to_account_id: impl Into<String>,
        amount: i64,
        asset_id: impl Into<String>,
    ) -> Self {
        Self {
            header,
            fee,
            issuer: AccountRefInput::new(issuer_id),
            issue_to_account: AccountRefInput::new(issue_to_account_id),
            asset_to_issue: AssetAmountInput::new(amount, asset_id),
        }
    }
}

pub trait AssetIssueAdapter {
    type Transaction;
    type Error;

    fn build_asset_issue_transaction(
        input: AssetIssueInput,
    ) -> Result<Self::Transaction, Self::Error>;
}

pub trait AssetIssueChainTypes: GrapheneOperationBuilderTypes {
    type AssetIssueOperation;

    fn asset_issue_operation(
        fee: Self::Asset,
        issuer: Self::AccountId,
        asset_to_issue: Self::Asset,
        issue_to_account: Self::AccountId,
        extensions: Self::FutureExtensions,
    ) -> Self::AssetIssueOperation;

    fn operation_asset_issue(operation: Self::AssetIssueOperation) -> Self::Operation;
}

pub fn build_asset_issue_transaction_for<C: AssetIssueChainTypes>(
    input: AssetIssueInput,
) -> C::Transaction {
    let fee = C::asset(input.fee.amount, input.fee.asset_id.into());
    let asset_to_issue = C::asset(
        input.asset_to_issue.amount,
        input.asset_to_issue.asset_id.into(),
    );
    let operation = C::asset_issue_operation(
        fee,
        input.issuer.id.into(),
        asset_to_issue,
        input.issue_to_account.id.into(),
        C::empty_extensions(),
    );

    C::transaction(
        input.header,
        vec![C::operation_asset_issue(operation)],
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
    fn asset_issue_constructor_wraps_raw_issue_fields() {
        let input = AssetIssueInput::new(
            header(),
            FeeInput::new(200_000, "1.3.0"),
            "1.2.100",
            "1.2.101",
            100_000,
            "1.3.1",
        );

        assert_eq!(input.issuer.id, "1.2.100");
        assert_eq!(input.issue_to_account.id, "1.2.101");
        assert_eq!(
            input.asset_to_issue,
            AssetAmountInput::new(100_000, "1.3.1")
        );
    }
}
