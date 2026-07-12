use graphene_chain_swaplock_bindings::generated::LimitOrderObject;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

/// Default page size when the caller does not set one.
pub const DEFAULT_GET_LIMIT_ORDERS_LIMIT: u32 = 100;

/// Builder for `get_limit_orders`: the raw order book for one market.
///
/// `base` is the asset you sell, `quote` the asset you buy, each an id like `1.3.0`. You get back
/// the resting orders for that pair, newest price levels first. For the grouped (aggregated) book
/// use [`OrdersApi::grouped_limit_orders`](crate::OrdersApi); for one account's own orders use
/// [`DatabaseApi::account_orders`](crate::DatabaseApi).
pub struct GetLimitOrdersRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) base: String,
    pub(super) quote: String,
    pub(super) limit: u32,
}

impl GetLimitOrdersRequest<'_> {
    /// Cap how many orders come back (node has its own maximum, usually 100).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub async fn get(self) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        let value = self
            .session
            .database_call(
                "get_limit_orders",
                json!([self.base, self.quote, self.limit]),
            )
            .await?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_limit_orders",
            message: error.to_string(),
        })
    }
}
