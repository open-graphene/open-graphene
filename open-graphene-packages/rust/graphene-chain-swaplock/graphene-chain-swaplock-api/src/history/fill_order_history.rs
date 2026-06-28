use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

pub const DEFAULT_FILL_ORDER_HISTORY_LIMIT: u32 = 100;
pub const MAX_FILL_ORDER_HISTORY_LIMIT: u32 = 200;

pub struct FillOrderHistoryRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) base: String,
    pub(super) quote: String,
    pub(super) limit: u32,
}

impl FillOrderHistoryRequest<'_> {
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub async fn get(self) -> Result<Vec<Value>, SwaplockApiError> {
        validate_fill_order_history_limit(self.limit)?;
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

fn validate_fill_order_history_limit(limit: u32) -> Result<(), SwaplockApiError> {
    if limit == 0 || limit > MAX_FILL_ORDER_HISTORY_LIMIT {
        return Err(SwaplockApiError::InvalidLimit {
            method: "get_fill_order_history",
            limit,
            max: MAX_FILL_ORDER_HISTORY_LIMIT,
        });
    }

    Ok(())
}
