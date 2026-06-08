//! Domain types for the Swaplock `orders` (grouped market orders) API.

use graphene_chain_swaplock_bindings::generated::Price;
use serde::Deserialize;

/// One price bucket of grouped limit orders in a market: the price range it
/// covers and the total amount (in the base asset) offered for sale within it.
#[derive(Debug, Clone, Deserialize)]
pub struct LimitOrderGroup {
    pub min_price: Price,
    pub max_price: Price,
    // `share_type` (int64) arrives as a JSON number or a decimal string; the shared
    // core helper accepts both rather than re-implementing the rule here.
    #[serde(
        deserialize_with = "open_graphene_core::deserialize_i64_from_number_or_decimal_string"
    )]
    pub total_for_sale: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn market(base_amount: i64, quote_amount: i64) -> serde_json::Value {
        json!({
            "base": {"amount": base_amount, "asset_id": "1.3.0"},
            "quote": {"amount": quote_amount, "asset_id": "1.3.1"}
        })
    }

    #[test]
    fn group_decodes_total_for_sale_from_number() {
        let group: LimitOrderGroup = serde_json::from_value(json!({
            "min_price": market(10, 1),
            "max_price": market(12, 1),
            "total_for_sale": 5000
        }))
        .unwrap();
        assert_eq!(group.total_for_sale, 5000);
        assert_eq!(group.min_price.base.asset_id.0, "1.3.0");
    }

    #[test]
    fn group_decodes_total_for_sale_from_decimal_string() {
        let group: LimitOrderGroup = serde_json::from_value(json!({
            "min_price": market(10, 1),
            "max_price": market(12, 1),
            "total_for_sale": "9223372036854775807"
        }))
        .unwrap();
        assert_eq!(group.total_for_sale, i64::MAX);
    }
}
