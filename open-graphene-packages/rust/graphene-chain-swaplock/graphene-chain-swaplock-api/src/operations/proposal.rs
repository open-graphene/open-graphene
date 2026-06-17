//! Ergonomic builder for `proposal_create`: wrap one or more operations in a proposal.
//!
//! This is the Rust take on bitsharesjs `propose`. You hand it operations built the usual way (via
//! the binding types), it prices each one, wraps them, and builds the proposal. Approving the
//! proposal later executes the wrapped ops under the required authorities.

use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, ProposalId};
use graphene_chain_swaplock_bindings::generated::operations::{
    ProposalCreateOperation, ProposalDeleteOperation, ProposalUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{Asset, OpWrapper};
use open_graphene_core::{HeadBlock, transaction_header_from_head};
use open_graphene_transport::GrapheneSession;

use crate::{DatabaseApi, SwaplockApiError};

use super::transaction::{
    PreparedTransaction, TransactionBuilder, required_fees, set_operation_fee,
};

/// How long the proposal stays open for approval if not set.
const DEFAULT_PROPOSAL_EXPIRATION: Duration = Duration::from_secs(3600);
const FEE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(FEE_ASSET_ID.to_string()))
}

/// Builder for `proposal_create`: `fee_paying_account` proposes the wrapped operations.
pub struct ProposalCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    fee_paying_account: String,
    proposed: Vec<Operation>,
    expiration: Duration,
    review_period: Option<Duration>,
}

impl<'session> ProposalCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        fee_paying_account: impl Into<String>,
    ) -> Self {
        Self {
            session,
            fee_paying_account: fee_paying_account.into(),
            proposed: Vec::new(),
            expiration: DEFAULT_PROPOSAL_EXPIRATION,
            review_period: None,
        }
    }

    /// Add an operation to the proposal. Call more than once to propose several at once. Build the
    /// operation from the binding types, e.g. `Operation::transfer(..)`, with its fee left at zero.
    pub fn propose(mut self, operation: Operation) -> Self {
        self.proposed.push(operation);
        self
    }

    /// How long the proposal stays open for approval, from the head block (default one hour).
    pub fn expiration(mut self, expiration: Duration) -> Self {
        self.expiration = expiration;
        self
    }

    /// Require a review window (in which approvals can still be withdrawn) before execution.
    pub fn review_period(mut self, review_period: Duration) -> Self {
        self.review_period = Some(review_period);
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        if self.proposed.is_empty() {
            return Err(SwaplockApiError::InvalidTransfer {
                message: "proposal has no operations".to_string(),
            });
        }

        // Price the wrapped operations: the proposal carries each op with its own fee set.
        let mut proposed = self.proposed;
        let fees = required_fees(self.session, &proposed, FEE_ASSET_ID)?;
        for (operation, fee) in proposed.iter_mut().zip(fees) {
            set_operation_fee(operation, fee)?;
        }

        let properties = DatabaseApi {
            session: &mut *self.session,
        }
        .get_dynamic_global_properties()
        .await?;
        let expiration_time = transaction_header_from_head(
            &HeadBlock {
                number: properties.head_block_number as u64,
                id: hex::encode(&properties.head_block_id),
                time: properties.time.clone(),
            },
            self.expiration,
        )?
        .expiration;

        let proposed_ops = proposed.into_iter().map(|op| OpWrapper { op }).collect();
        let operation = Operation::proposal_create(ProposalCreateOperation {
            fee: Asset::new(0, AssetId(FEE_ASSET_ID.to_string())),
            fee_paying_account: AccountId(self.fee_paying_account),
            expiration_time,
            proposed_ops,
            review_period_seconds: self.review_period.map(|period| period.as_secs() as u32),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `proposal_update`: add or remove approvals on a live proposal.
///
/// Required: the `fee_paying_account` and the `proposal` id. Use the setters to add or remove active,
/// owner or key approvals. This is how a party signs off on (or revokes) a multisig proposal.
pub struct ProposalUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    fee_paying_account: String,
    proposal: String,
    active_to_add: Vec<String>,
    active_to_remove: Vec<String>,
    owner_to_add: Vec<String>,
    owner_to_remove: Vec<String>,
    key_to_add: Vec<String>,
    key_to_remove: Vec<String>,
}

impl<'session> ProposalUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        fee_paying_account: impl Into<String>,
        proposal: impl Into<String>,
    ) -> Self {
        Self {
            session,
            fee_paying_account: fee_paying_account.into(),
            proposal: proposal.into(),
            active_to_add: vec![],
            active_to_remove: vec![],
            owner_to_add: vec![],
            owner_to_remove: vec![],
            key_to_add: vec![],
            key_to_remove: vec![],
        }
    }

    /// Add an account's active-authority approval.
    pub fn approve_active(mut self, account: impl Into<String>) -> Self {
        self.active_to_add.push(account.into());
        self
    }

    /// Withdraw an account's active-authority approval.
    pub fn unapprove_active(mut self, account: impl Into<String>) -> Self {
        self.active_to_remove.push(account.into());
        self
    }

    /// Add an account's owner-authority approval.
    pub fn approve_owner(mut self, account: impl Into<String>) -> Self {
        self.owner_to_add.push(account.into());
        self
    }

    /// Withdraw an account's owner-authority approval.
    pub fn unapprove_owner(mut self, account: impl Into<String>) -> Self {
        self.owner_to_remove.push(account.into());
        self
    }

    /// Add a key approval (for an authority that names a key directly).
    pub fn approve_key(mut self, key: impl Into<String>) -> Self {
        self.key_to_add.push(key.into());
        self
    }

    /// Withdraw a key approval.
    pub fn unapprove_key(mut self, key: impl Into<String>) -> Self {
        self.key_to_remove.push(key.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let to_accounts = |names: Vec<String>| names.into_iter().map(AccountId).collect();
        let operation = Operation::proposal_update(ProposalUpdateOperation {
            fee: core_fee(),
            fee_paying_account: AccountId(self.fee_paying_account),
            proposal: ProposalId(self.proposal),
            active_approvals_to_add: to_accounts(self.active_to_add),
            active_approvals_to_remove: to_accounts(self.active_to_remove),
            owner_approvals_to_add: to_accounts(self.owner_to_add),
            owner_approvals_to_remove: to_accounts(self.owner_to_remove),
            key_approvals_to_add: self.key_to_add,
            key_approvals_to_remove: self.key_to_remove,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `proposal_delete`: drop a proposal before it executes.
///
/// Required: the `fee_paying_account` and the `proposal` id. Pass `.using_owner_authority(true)` when
/// deleting via owner rather than active authority.
pub struct ProposalDeleteRequest<'session> {
    session: &'session mut GrapheneSession,
    fee_paying_account: String,
    proposal: String,
    using_owner_authority: bool,
}

impl<'session> ProposalDeleteRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        fee_paying_account: impl Into<String>,
        proposal: impl Into<String>,
    ) -> Self {
        Self {
            session,
            fee_paying_account: fee_paying_account.into(),
            proposal: proposal.into(),
            using_owner_authority: false,
        }
    }

    /// Delete using the owner authority instead of the active authority.
    pub fn using_owner_authority(mut self, using_owner_authority: bool) -> Self {
        self.using_owner_authority = using_owner_authority;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::proposal_delete(ProposalDeleteOperation {
            fee: core_fee(),
            fee_paying_account: AccountId(self.fee_paying_account),
            using_owner_authority: self.using_owner_authority,
            proposal: ProposalId(self.proposal),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
