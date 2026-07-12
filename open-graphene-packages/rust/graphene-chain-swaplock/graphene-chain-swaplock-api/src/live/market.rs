//! Live order-book subscription for a single market (`subscribe_to_market` / `unsubscribe_from_market`).
//!
//! `subscribe_to_market` registers a callback the node fires whenever the `base`/`quote` order book
//! changes; the updates arrive as raw JSON (the market notice shape is not in the typed binding set).
//! Dropping the subscription unsubscribes the market.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use open_graphene_transport::{CallbackId, LiveSubscription, LiveTransportHandle};
use serde_json::{Value, json};

use crate::SwaplockApiError;

/// Market callbacks use their own id range so they never collide with the shared object/history
/// callback (`LIVE_DATABASE_CALLBACK_ID`); each subscription gets a fresh id so notices demux.
const MARKET_CALLBACK_BASE: u64 =
    open_graphene_transport::SUBSCRIPTION_CALLBACK_ID_BASE + 1_000_000;
static MARKET_CALLBACK_SEQ: AtomicU64 = AtomicU64::new(0);

fn next_market_callback_id() -> CallbackId {
    CallbackId::new(MARKET_CALLBACK_BASE + MARKET_CALLBACK_SEQ.fetch_add(1, Ordering::Relaxed))
}

/// A live subscription to a market's order book. Each [`next`](Self::next) yields the raw notice the
/// node pushed when the book changed. Dropping it sends `unsubscribe_from_market`.
pub struct SwaplockLiveMarketSubscription {
    base: String,
    quote: String,
    database_api_id: u64,
    live: LiveTransportHandle,
    subscription: LiveSubscription,
}

impl SwaplockLiveMarketSubscription {
    pub(super) async fn start(
        live: LiveTransportHandle,
        database_api_id: u64,
        base: String,
        quote: String,
        timeout: Duration,
    ) -> Result<Self, SwaplockApiError> {
        let callback_id = next_market_callback_id();
        let subscription = live.subscribe_callback(callback_id).await?;
        live.call(
            database_api_id,
            "subscribe_to_market",
            json!([callback_id.as_u64(), base, quote]),
        )?
        .wait_timeout(timeout)
        .await?;

        Ok(Self {
            base,
            quote,
            database_api_id,
            live,
            subscription,
        })
    }

    /// The market pair this subscription tracks (`base`, `quote`).
    pub fn market(&self) -> (&str, &str) {
        (&self.base, &self.quote)
    }

    /// Await the next order-book update (raw JSON).
    pub async fn next(&mut self) -> Result<Value, SwaplockApiError> {
        Ok(self.subscription.next().await?)
    }

    /// Await the next order-book update, giving up after `timeout`.
    pub async fn next_timeout(&mut self, timeout: Duration) -> Result<Value, SwaplockApiError> {
        Ok(self.subscription.next_timeout(timeout).await?)
    }
}

impl Drop for SwaplockLiveMarketSubscription {
    fn drop(&mut self) {
        // Fire-and-forget: queue the unsubscribe on the dispatcher; we cannot await in Drop.
        let _ = self.live.call(
            self.database_api_id,
            "unsubscribe_from_market",
            json!([self.base, self.quote]),
        );
    }
}
