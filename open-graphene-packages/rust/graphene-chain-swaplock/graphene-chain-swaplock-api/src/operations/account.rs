//! Ergonomic builder for `account_update`, scoped to the account's voting/options.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). The chain op
//! replaces the whole `account_options` block, so this builder fetches your current options first
//! and applies only the fields you set, leaving the rest untouched. Owner/active authority changes
//! are deliberately out of scope here (build those by hand to avoid locking yourself out).

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, VoteId};
use graphene_chain_swaplock_bindings::generated::operations::AccountUpdateOperation;
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{AccountUpdateOperationExt, Asset};
use open_graphene_transport::GrapheneSession;

use crate::{DatabaseApi, SwaplockApiError};

use super::transaction::{PreparedTransaction, TransactionBuilder};

/// Builder for `account_update`: change an account's voting options in place.
///
/// Only the setters you call move; everything else is carried over from the account's current
/// options. Nothing set means no change, which the node will reject, so set at least one.
pub struct AccountUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    memo_key: Option<String>,
    voting_account: Option<String>,
    num_witness: Option<u16>,
    num_committee: Option<u16>,
    votes: Option<Vec<String>>,
}

impl<'session> AccountUpdateRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, account: impl Into<String>) -> Self {
        Self {
            session,
            account: account.into(),
            memo_key: None,
            voting_account: None,
            num_witness: None,
            num_committee: None,
            votes: None,
        }
    }

    /// Set the memo key (used to encrypt memos, not for spending or authority).
    pub fn memo_key(mut self, memo_key: impl Into<String>) -> Self {
        self.memo_key = Some(memo_key.into());
        self
    }

    /// Set the proxy account your votes follow (`1.2.5` is "no proxy", vote for yourself).
    pub fn voting_account(mut self, account_id: impl Into<String>) -> Self {
        self.voting_account = Some(account_id.into());
        self
    }

    /// How many witnesses your votes should elect.
    pub fn num_witness(mut self, num_witness: u16) -> Self {
        self.num_witness = Some(num_witness);
        self
    }

    /// How many committee members your votes should elect.
    pub fn num_committee(mut self, num_committee: u16) -> Self {
        self.num_committee = Some(num_committee);
        self
    }

    /// Replace the set of things you vote for (vote ids like `1:26`).
    pub fn votes<I, S>(mut self, votes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.votes = Some(votes.into_iter().map(Into::into).collect());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let mut options = DatabaseApi {
            session: &mut *self.session,
        }
        .get_account_by_id(&self.account)
        .await?
        .options;

        if let Some(memo_key) = self.memo_key {
            options.memo_key = memo_key;
        }
        if let Some(voting_account) = self.voting_account {
            options.voting_account = AccountId(voting_account);
        }
        if let Some(num_witness) = self.num_witness {
            options.num_witness = num_witness;
        }
        if let Some(num_committee) = self.num_committee {
            options.num_committee = num_committee;
        }
        if let Some(votes) = self.votes {
            options.votes = votes.into_iter().map(VoteId).collect();
        }

        let operation = Operation::account_update(AccountUpdateOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            account: AccountId(self.account),
            owner: None,
            active: None,
            new_options: Some(options),
            extensions: AccountUpdateOperationExt {
                null_ext: None,
                owner_special_authority: None,
                active_special_authority: None,
            },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
