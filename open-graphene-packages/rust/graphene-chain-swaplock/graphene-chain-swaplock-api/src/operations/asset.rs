//! Ergonomic builders for the issuer-side asset supply operations: issue and reserve.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). Both are
//! issuer moves: `issue` mints new units of a user asset to an account, `reserve` burns units back
//! out of existence (only the holder can reserve their own).

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_swaplock_bindings::generated::operations::{
    AssetIssueOperation, AssetReserveOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

/// Builder for `asset_issue`: the asset's issuer mints `amount` and sends it to an account.
pub struct AssetIssueRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    asset_to_issue: Option<(i64, String)>,
    issue_to_account: Option<String>,
}

impl<'session> AssetIssueRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, issuer: impl Into<String>) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            asset_to_issue: None,
            issue_to_account: None,
        }
    }

    /// How much to mint: raw `amount` of asset id `asset_id` (must be an asset you issue).
    pub fn issue(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.asset_to_issue = Some((amount, asset_id.into()));
        self
    }

    /// Who receives the freshly minted units.
    pub fn to(mut self, account_id: impl Into<String>) -> Self {
        self.issue_to_account = Some(account_id.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset_id) = self
            .asset_to_issue
            .ok_or(SwaplockApiError::MissingTransferField { field: "issue" })?;
        let issue_to_account = self
            .issue_to_account
            .ok_or(SwaplockApiError::MissingTransferField { field: "to" })?;

        let operation = Operation::asset_issue(AssetIssueOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            issuer: AccountId(self.issuer),
            asset_to_issue: Asset::new(amount, AssetId(asset_id)),
            issue_to_account: AccountId(issue_to_account),
            memo: None,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_reserve`: burn `amount` you hold back out of the supply.
pub struct AssetReserveRequest<'session> {
    session: &'session mut GrapheneSession,
    payer: String,
    amount_to_reserve: Option<(i64, String)>,
}

impl<'session> AssetReserveRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, payer: impl Into<String>) -> Self {
        Self {
            session,
            payer: payer.into(),
            amount_to_reserve: None,
        }
    }

    /// How much to burn: raw `amount` of asset id `asset_id` from the payer's balance.
    pub fn amount(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.amount_to_reserve = Some((amount, asset_id.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset_id) = self
            .amount_to_reserve
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;

        let operation = Operation::asset_reserve(AssetReserveOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            payer: AccountId(self.payer),
            amount_to_reserve: Asset::new(amount, AssetId(asset_id)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
