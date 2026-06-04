//! Low-level `orders` (grouped market orders) API calls.
//!
//! This layer only shapes JSON-RPC params and forwards them; typed market
//! results live in the higher SDK crate. The `start` price arrives already
//! encoded as JSON (an object, or `null` when unset).

use serde_json::{Value, json};

use crate::{GrapheneSession, TransportError};

pub fn get_tracked_groups(session: &mut GrapheneSession) -> Result<Value, TransportError> {
    session.orders_call("get_tracked_groups", json!([]))
}

pub fn get_grouped_limit_orders(
    session: &mut GrapheneSession,
    base_asset: &str,
    quote_asset: &str,
    group: u16,
    start: Value,
    limit: u32,
) -> Result<Value, TransportError> {
    session.orders_call(
        "get_grouped_limit_orders",
        get_grouped_limit_orders_params(base_asset, quote_asset, group, start, limit),
    )
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
