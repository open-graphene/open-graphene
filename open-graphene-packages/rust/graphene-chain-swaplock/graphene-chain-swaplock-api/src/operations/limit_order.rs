//! Ergonomic builders for the market operations: place and cancel a limit order.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder): you give the
//! amounts and ids, it builds the typed operation, prices it, and hands back a transaction to sign.

use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, LimitOrderId};
use graphene_chain_swaplock_bindings::generated::operations::{
    LimitOrderCancelOperation, LimitOrderCreateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_core::{HeadBlock, transaction_header_from_head};
use open_graphene_transport::GrapheneSession;

use crate::{DatabaseApi, SwaplockApiError};

use super::transaction::{PreparedTransaction, TransactionBuilder};

/// How long the order rests on the book if not set (the node also caps this).
const DEFAULT_ORDER_DURATION: Duration = Duration::from_secs(3600);

/// Builder for `limit_order_create`: offer `sell` for at least `receive`.
pub struct LimitOrderCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    seller: String,
    sell: Option<(i64, String)>,
    receive: Option<(i64, String)>,
    fill_or_kill: bool,
    order_duration: Duration,
}

impl<'session> LimitOrderCreateRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, seller: impl Into<String>) -> Self {
        Self {
            session,
            seller: seller.into(),
            sell: None,
            receive: None,
            fill_or_kill: false,
            order_duration: DEFAULT_ORDER_DURATION,
        }
    }

    /// What you put up: raw `amount` of asset id `asset_id` (e.g. `1.3.0`).
    pub fn sell(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.sell = Some((amount, asset_id.into()));
        self
    }

    /// The minimum you accept in return: raw `amount` of asset id `asset_id`.
    pub fn receive(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.receive = Some((amount, asset_id.into()));
        self
    }

    /// Cancel instead of resting if it can't fill fully right away.
    pub fn fill_or_kill(mut self, fill_or_kill: bool) -> Self {
        self.fill_or_kill = fill_or_kill;
        self
    }

    /// How long the order may rest on the book (default one hour).
    pub fn order_duration(mut self, order_duration: Duration) -> Self {
        self.order_duration = order_duration;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (sell_amount, sell_asset) = self
            .sell
            .ok_or(SwaplockApiError::MissingTransferField { field: "sell" })?;
        let (receive_amount, receive_asset) = self
            .receive
            .ok_or(SwaplockApiError::MissingTransferField { field: "receive" })?;

        let properties = DatabaseApi {
            session: &mut *self.session,
        }
        .get_dynamic_global_properties()
        .await?;
        let expiration = transaction_header_from_head(
            &HeadBlock {
                number: properties.head_block_number as u64,
                id: hex(&properties.head_block_id),
                time: properties.time.clone(),
            },
            self.order_duration,
        )?
        .expiration;

        let operation = Operation::limit_order_create(LimitOrderCreateOperation {
            fee: Asset::new(0, AssetId(sell_asset.clone())),
            seller: AccountId(self.seller),
            amount_to_sell: Asset::new(sell_amount, AssetId(sell_asset)),
            min_to_receive: Asset::new(receive_amount, AssetId(receive_asset)),
            expiration,
            fill_or_kill: self.fill_or_kill,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `limit_order_cancel`: take an order off the book by its id.
pub struct LimitOrderCancelRequest<'session> {
    session: &'session mut GrapheneSession,
    fee_paying_account: String,
    order: String,
}

impl<'session> LimitOrderCancelRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        fee_paying_account: impl Into<String>,
        order: impl Into<String>,
    ) -> Self {
        Self {
            session,
            fee_paying_account: fee_paying_account.into(),
            order: order.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::limit_order_cancel(LimitOrderCancelOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            fee_paying_account: AccountId(self.fee_paying_account),
            order: LimitOrderId(self.order),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
