use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

/// Default number of fills returned when the caller does not set a limit.
pub const DEFAULT_FILL_ORDER_HISTORY_LIMIT: u32 = 100;

/// Builder for `get_fill_order_history`: the most recent trades (filled orders) on the `base`/`quote`
/// market, newest first.
///
/// `.limit(..)` caps how many fills come back (default 100). The fills come back as raw JSON, like
/// [`get_objects`](crate::DatabaseApi::objects): the market-history plugin's order-history objects
/// are not in the typed binding set.
pub struct FillOrderHistoryRequest<'session> {
    session: &'session mut GrapheneSession,
    base: String,
    quote: String,
    limit: u32,
}

impl<'session> FillOrderHistoryRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        base: impl Into<String>,
        quote: impl Into<String>,
    ) -> Self {
        Self {
            session,
            base: base.into(),
            quote: quote.into(),
            limit: DEFAULT_FILL_ORDER_HISTORY_LIMIT,
        }
    }

    /// How many fills to return, newest first (default 100).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub async fn get(self) -> Result<Vec<Value>, SwaplockApiError> {
        let value = self
            .session
            .history_call(
                "get_fill_order_history",
                json!([self.base, self.quote, self.limit]),
            )
            .await?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_fill_order_history",
            message: error.to_string(),
        })
    }
}
