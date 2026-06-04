//! Domain types for the Swaplock `orders` (grouped market orders) API.

use graphene_chain_swaplock_bindings::generated::Price;
use serde::Deserialize;

/// One price bucket of grouped limit orders in a market: the price range it
/// covers and the total amount (in the base asset) offered for sale within it.
#[derive(Debug, Clone, Deserialize)]
pub struct LimitOrderGroup {
    pub min_price: Price,
    pub max_price: Price,
    #[serde(deserialize_with = "deserialize_i64_lenient")]
    pub total_for_sale: i64,
}

/// `share_type` (int64) is sometimes serialized as a JSON number and sometimes as
/// a decimal string; accept both.
fn deserialize_i64_lenient<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumberOrString {
        Number(i64),
        Text(String),
    }

    match NumberOrString::deserialize(deserializer)? {
        NumberOrString::Number(value) => Ok(value),
        NumberOrString::Text(text) => text.trim().parse().map_err(serde::de::Error::custom),
    }
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
