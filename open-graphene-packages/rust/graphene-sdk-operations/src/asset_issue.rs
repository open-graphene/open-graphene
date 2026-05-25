use open_graphene_sdk_core::TransactionHeader;

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
