use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

/// One hour, the default candle width when the caller does not set one.
pub const DEFAULT_MARKET_HISTORY_BUCKET_SECONDS: u32 = 3600;

/// Builder for `get_market_history`: OHLC price candles for the `base`/`quote` market.
///
/// Required: the `.range(start, end)` window (timestamps `YYYY-MM-DDThh:mm:ss`). `.bucket_seconds(..)`
/// sets the candle width (default one hour; the node only serves widths it tracks). The buckets come
/// back as raw JSON, like [`get_objects`](crate::DatabaseApi::objects): the market-history plugin's
/// bucket objects are not in the typed binding set.
pub struct MarketHistoryRequest<'session> {
    session: &'session mut GrapheneSession,
    base: String,
    quote: String,
    bucket_seconds: u32,
    range: Option<(String, String)>,
}

impl<'session> MarketHistoryRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        base: impl Into<String>,
        quote: impl Into<String>,
    ) -> Self {
        Self {
            session,
            base: base.into(),
            quote: quote.into(),
            bucket_seconds: DEFAULT_MARKET_HISTORY_BUCKET_SECONDS,
            range: None,
        }
    }

    /// Candle width in seconds (default one hour). The node only serves widths it is configured for.
    pub fn bucket_seconds(mut self, bucket_seconds: u32) -> Self {
        self.bucket_seconds = bucket_seconds;
        self
    }

    /// The time window to fetch, as `YYYY-MM-DDThh:mm:ss` timestamps.
    pub fn range(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.range = Some((start.into(), end.into()));
        self
    }

    pub async fn get(self) -> Result<Vec<Value>, SwaplockApiError> {
        let (start, end) = self
            .range
            .ok_or(SwaplockApiError::MissingTransferField { field: "range" })?;
        let value = self
            .session
            .history_call(
                "get_market_history",
                json!([self.base, self.quote, self.bucket_seconds, start, end]),
            )
            .await?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_market_history",
            message: error.to_string(),
        })
    }
}
