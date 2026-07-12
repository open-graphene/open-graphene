//! Ergonomic builder for `balance_claim`: claim a genesis/imported balance into an account.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder).

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, BalanceId};
use graphene_chain_swaplock_bindings::generated::operations::BalanceClaimOperation;
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Builder for `balance_claim`: move a claimable balance object into `deposit_to`.
///
/// Required: the `deposit_to` account, the `balance` object id, the `owner_key` that controls the
/// balance (a public key string) and the `.amount(..)` claimed (must equal the balance's value).
pub struct BalanceClaimRequest<'session> {
    session: &'session mut GrapheneSession,
    deposit_to: String,
    balance: String,
    owner_key: String,
    amount: Option<(i64, String)>,
}

impl<'session> BalanceClaimRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        deposit_to: impl Into<String>,
        balance: impl Into<String>,
        owner_key: impl Into<String>,
    ) -> Self {
        Self {
            session,
            deposit_to: deposit_to.into(),
            balance: balance.into(),
            owner_key: owner_key.into(),
            amount: None,
        }
    }

    /// How much to claim, in raw units of `asset` (must match the balance object's amount).
    pub fn amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.amount = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let operation = Operation::balance_claim(BalanceClaimOperation {
            fee: core_fee(),
            deposit_to_account: AccountId(self.deposit_to),
            balance_to_claim: BalanceId(self.balance),
            balance_owner_key: self.owner_key,
            total_claimed: Asset::new(amount, AssetId(asset)),
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
