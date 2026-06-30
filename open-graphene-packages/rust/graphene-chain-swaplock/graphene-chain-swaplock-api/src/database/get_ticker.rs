use open_graphene_transport::GrapheneSession;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::SwaplockApiError;

/// Result returned by Graphene `get_ticker` for one market pair.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Ticker {
    pub time: String,
    pub base: String,
    pub quote: String,
    pub latest: String,
    pub lowest_ask: String,
    pub highest_bid: String,
    pub percent_change: String,
    pub base_volume: String,
    pub quote_volume: String,
}

/// Builder for `get_ticker`: rolling market statistics for one pair.
pub struct GetTickerRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) base: String,
    pub(super) quote: String,
}

impl GetTickerRequest<'_> {
    pub async fn get(self) -> Result<Ticker, SwaplockApiError> {
        let value = self
            .session
            .database_call("get_ticker", json!([self.base, self.quote]))
            .await?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_ticker",
            message: error.to_string(),
        })
    }
}
