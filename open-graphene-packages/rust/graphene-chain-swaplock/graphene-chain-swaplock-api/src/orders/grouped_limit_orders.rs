use graphene_chain_swaplock_bindings::generated::Price;
use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::types::LimitOrderGroup;
use crate::codec::{decode, encode};

/// Default page size when the caller does not set one.
pub const DEFAULT_GROUPED_LIMIT_ORDERS_LIMIT: u32 = 100;

/// Builder for `orders.get_grouped_limit_orders`: grouped order book for a market,
/// ordered from best to worst price. `start` and `limit` are optional.
pub struct GroupedLimitOrdersRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) base_asset: String,
    pub(super) quote_asset: String,
    pub(super) group: u16,
    pub(super) start: Option<Price>,
    pub(super) limit: u32,
}

impl GroupedLimitOrdersRequest<'_> {
    /// Start enumerating from this price (best offered price by default).
    pub fn start(mut self, start: Price) -> Self {
        self.start = Some(start);
        self
    }

    /// Cap the number of returned groups (node enforces its own maximum).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub async fn get(self) -> Result<Vec<LimitOrderGroup>, SwaplockApiError> {
        get_grouped_limit_orders(
            self.session,
            &self.base_asset,
            &self.quote_asset,
            self.group,
            self.start,
            self.limit,
        )
        .await
    }
}

pub(super) async fn get_grouped_limit_orders(
    session: &mut GrapheneSession,
    base_asset: &str,
    quote_asset: &str,
    group: u16,
    start: Option<Price>,
    limit: u32,
) -> Result<Vec<LimitOrderGroup>, SwaplockApiError> {
    let start = match start {
        Some(price) => encode("get_grouped_limit_orders", &price)?,
        None => Value::Null,
    };
    let result = session
        .orders_call(
            "get_grouped_limit_orders",
            get_grouped_limit_orders_params(base_asset, quote_asset, group, start, limit),
        )
        .await?;
    decode("get_grouped_limit_orders", result)
}

fn get_grouped_limit_orders_params(
    base_asset: &str,
    quote_asset: &str,
    group: u16,
    start: Value,
    limit: u32,
) -> Value {
    json!([base_asset, quote_asset, group, start, limit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_limit_orders_params_match_node_argument_order() {
        assert_eq!(
            get_grouped_limit_orders_params("BTS", "USD", 10, Value::Null, 50),
            json!(["BTS", "USD", 10, null, 50])
        );
    }

    #[test]
    fn grouped_limit_orders_params_pass_start_price_when_present() {
        let start = json!({
            "base": {"amount": 100, "asset_id": "1.3.0"},
            "quote": {"amount": 1, "asset_id": "1.3.1"}
        });
        assert_eq!(
            get_grouped_limit_orders_params("BTS", "USD", 10, start.clone(), 50),
            json!(["BTS", "USD", 10, start, 50])
        );
    }
}
