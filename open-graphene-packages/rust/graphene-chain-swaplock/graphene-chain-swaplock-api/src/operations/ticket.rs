//! Ergonomic builders for stake tickets: lock funds into a voting-power tier, or move an existing
//! ticket to a different tier.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). `target_type`
//! selects the tier (0 liquid, 1 lock 180 days, 2 lock 360 days, 3 lock 720 days).

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, TicketId};
use graphene_chain_swaplock_bindings::generated::operations::{
    TicketCreateOperation, TicketUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Builder for `ticket_create`: lock `amount` into the `target_type` tier.
///
/// Required: the `account`, the `target_type` tier and the `.amount(..)` to lock.
pub struct TicketCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    target_type: u32,
    amount: Option<(i64, String)>,
}

impl<'session> TicketCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        target_type: u32,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            target_type,
            amount: None,
        }
    }

    /// How much to lock into the ticket, in raw units of `asset`.
    pub fn amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.amount = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let operation = Operation::ticket_create(TicketCreateOperation {
            fee: core_fee(),
            account: AccountId(self.account),
            target_type: u64::from(self.target_type),
            amount: Asset::new(amount, AssetId(asset)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `ticket_update`: move a ticket to a different tier.
///
/// Required: the `ticket` id, the `account` and the new `target_type`. Set `.amount(..)` to move only
/// part of the ticket into the new tier; omit it to move the whole ticket.
pub struct TicketUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    ticket: String,
    account: String,
    target_type: u32,
    amount_for_new_target: Option<(i64, String)>,
}

impl<'session> TicketUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        ticket: impl Into<String>,
        account: impl Into<String>,
        target_type: u32,
    ) -> Self {
        Self {
            session,
            ticket: ticket.into(),
            account: account.into(),
            target_type,
            amount_for_new_target: None,
        }
    }

    /// Move only this much into the new tier, in raw units of `asset` (omit to move the whole ticket).
    pub fn amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.amount_for_new_target = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::ticket_update(TicketUpdateOperation {
            fee: core_fee(),
            ticket: TicketId(self.ticket),
            account: AccountId(self.account),
            target_type: u64::from(self.target_type),
            amount_for_new_target: self
                .amount_for_new_target
                .map(|(amount, asset)| Asset::new(amount, AssetId(asset))),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
