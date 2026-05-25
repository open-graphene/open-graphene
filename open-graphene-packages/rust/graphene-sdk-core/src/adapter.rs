use crate::{AccountCreateInput, AssetCreateInput, AssetIssueInput, TransferInput};

pub trait TransferAdapter {
    type Transaction;
    type Error;

    fn build_transfer_transaction(input: TransferInput) -> Result<Self::Transaction, Self::Error>;
}

pub trait AccountCreateAdapter {
    type Transaction;
    type Error;

    fn build_account_create_transaction(
        input: AccountCreateInput,
    ) -> Result<Self::Transaction, Self::Error>;
}

pub trait AssetIssueAdapter {
    type Transaction;
    type Error;

    fn build_asset_issue_transaction(
        input: AssetIssueInput,
    ) -> Result<Self::Transaction, Self::Error>;
}

pub trait AssetCreateAdapter {
    type Transaction;
    type Error;

    fn build_asset_create_transaction(
        input: AssetCreateInput,
    ) -> Result<Self::Transaction, Self::Error>;
}
