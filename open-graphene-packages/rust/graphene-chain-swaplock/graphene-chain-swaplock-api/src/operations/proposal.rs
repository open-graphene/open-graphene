//! Ergonomic builder for `proposal_create`: wrap one or more operations in a proposal.
//!
//! This is the Rust take on bitsharesjs `propose`. You hand it operations built the usual way (via
//! the binding types), it prices each one, wraps them, and builds the proposal. Approving the
//! proposal later executes the wrapped ops under the required authorities.

use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_swaplock_bindings::generated::operations::ProposalCreateOperation;
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
