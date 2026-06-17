//! Ergonomic builder for `account_update`, scoped to the account's voting/options.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). The chain op
//! replaces the whole `account_options` block, so this builder fetches your current options first
//! and applies only the fields you set, leaving the rest untouched. Owner/active authority changes
//! are deliberately out of scope here (build those by hand to avoid locking yourself out).

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, VoteId};
use graphene_chain_swaplock_bindings::generated::operations::{
    AccountCreateOperation, AccountTransferOperation, AccountUpdateOperation,
    AccountUpgradeOperation, AccountWhitelistOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{
    AccountCreateOperationExt, AccountOptions, AccountUpdateOperationExt, Asset, Authority,
};
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

/// Build a single-key authority (one key, weight 1, threshold 1). Used for account creation and for
/// the owner of a blind output/input.
pub(super) fn single_key_authority(key: String) -> Authority {
    Authority {
        weight_threshold: 1,
        account_auths: vec![],
        key_auths: vec![(key, 1)],
        address_auths: vec![],
    }
}

/// Builder for `account_create`: register a new account with single-key owner/active authorities.
///
/// `.keys(k)` sets owner, active and memo to the one public key; override any with the matching
/// setter. The new account votes for nobody and follows the default proxy.
pub struct AccountCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    registrar: String,
    name: String,
    owner_key: Option<String>,
    active_key: Option<String>,
    memo_key: Option<String>,
    referrer: Option<String>,
    referrer_percent: u16,
}

impl<'session> AccountCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        registrar: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            session,
            registrar: registrar.into(),
            name: name.into(),
            owner_key: None,
            active_key: None,
            memo_key: None,
            referrer: None,
            referrer_percent: 0,
        }
    }

    /// Set owner and active (and, unless overridden, memo) to this one public key.
    pub fn keys(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.owner_key = Some(key.clone());
        self.active_key = Some(key);
        self
    }

    /// Override the owner key.
    pub fn owner_key(mut self, key: impl Into<String>) -> Self {
        self.owner_key = Some(key.into());
        self
    }

    /// Override the active key.
    pub fn active_key(mut self, key: impl Into<String>) -> Self {
        self.active_key = Some(key.into());
        self
    }

    /// Override the memo key (defaults to the active key).
    pub fn memo_key(mut self, key: impl Into<String>) -> Self {
        self.memo_key = Some(key.into());
        self
    }

    /// The referring account and its cut in hundredths of a percent (defaults to the registrar, 0%).
    pub fn referrer(mut self, referrer: impl Into<String>, percent: u16) -> Self {
        self.referrer = Some(referrer.into());
        self.referrer_percent = percent;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let owner_key = self
            .owner_key
            .ok_or(SwaplockApiError::MissingTransferField { field: "owner_key" })?;
        let active_key = self
            .active_key
            .ok_or(SwaplockApiError::MissingTransferField {
                field: "active_key",
            })?;
        let memo_key = self.memo_key.unwrap_or_else(|| active_key.clone());
        let referrer = self.referrer.unwrap_or_else(|| self.registrar.clone());

        let operation = Operation::account_create(AccountCreateOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            registrar: AccountId(self.registrar),
            referrer: AccountId(referrer),
            referrer_percent: self.referrer_percent,
            name: self.name,
            owner: single_key_authority(owner_key),
            active: single_key_authority(active_key),
            options: AccountOptions {
                memo_key,
                voting_account: AccountId("1.2.5".to_string()),
                num_witness: 0,
                num_committee: 0,
                votes: vec![],
                extensions: vec![],
            },
            extensions: AccountCreateOperationExt {
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

/// Builder for `account_upgrade`: turn an account into a lifetime member.
pub struct AccountUpgradeRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    upgrade_to_lifetime_member: bool,
}

impl<'session> AccountUpgradeRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, account: impl Into<String>) -> Self {
        Self {
            session,
            account: account.into(),
            upgrade_to_lifetime_member: true,
        }
    }

    /// Whether to upgrade to lifetime membership (default true).
    pub fn lifetime_member(mut self, upgrade: bool) -> Self {
        self.upgrade_to_lifetime_member = upgrade;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::account_upgrade(AccountUpgradeOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            account_to_upgrade: AccountId(self.account),
            upgrade_to_lifetime_member: self.upgrade_to_lifetime_member,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `account_whitelist`: `authorizing_account` lists `account_to_list`.
///
/// The listing flag is a bitfield: `0` no listing, `1` white-listed, `2` black-listed.
pub struct AccountWhitelistRequest<'session> {
    session: &'session mut GrapheneSession,
    authorizing_account: String,
    account_to_list: String,
    new_listing: u8,
}

impl<'session> AccountWhitelistRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        authorizing_account: impl Into<String>,
        account_to_list: impl Into<String>,
    ) -> Self {
        Self {
            session,
            authorizing_account: authorizing_account.into(),
            account_to_list: account_to_list.into(),
            new_listing: 0,
        }
    }

    /// Set the raw listing bitfield (0 none, 1 white, 2 black).
    pub fn listing(mut self, new_listing: u8) -> Self {
        self.new_listing = new_listing;
        self
    }

    /// White-list the account.
    pub fn white_listed(mut self) -> Self {
        self.new_listing = 1;
        self
    }

    /// Black-list the account.
    pub fn black_listed(mut self) -> Self {
        self.new_listing = 2;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::account_whitelist(AccountWhitelistOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            authorizing_account: AccountId(self.authorizing_account),
            account_to_list: AccountId(self.account_to_list),
            new_listing: self.new_listing,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `account_transfer`: hand an account over to a new owner account.
///
/// This gives away control of the account; use with care.
pub struct AccountTransferRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    new_owner: String,
}

impl<'session> AccountTransferRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        new_owner: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            new_owner: new_owner.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::account_transfer(AccountTransferOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            account_id: AccountId(self.account),
            new_owner: AccountId(self.new_owner),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
