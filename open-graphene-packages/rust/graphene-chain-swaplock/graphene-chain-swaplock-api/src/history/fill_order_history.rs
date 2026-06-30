use graphene_chain_swaplock_bindings::generated::{
    ids::{AssetId, ObjectId},
    operations::FillOrderOperation,
};
use open_graphene_transport::GrapheneSession;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::SwaplockApiError;

/// Default number of fills returned when the caller does not set a limit.
pub const DEFAULT_FILL_ORDER_HISTORY_LIMIT: u32 = 100;
pub const MAX_FILL_ORDER_HISTORY_LIMIT: u32 = 200;

/// Market-history plugin object returned by `history_api.get_fill_order_history`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = GrapheneSwaplockOrderHistoryObject)]
pub struct OrderHistoryObject {
    /// Plugin object ID, e.g. `5.0.199986569`.
    pub id: ObjectId,
    pub key: HistoryKey,
    pub time: String,
    pub op: FillOrderOperation,
}

/// Key used by the market-history plugin to order fill history by market pair and sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = GrapheneSwaplockHistoryKey)]
pub struct HistoryKey {
    pub base: AssetId,
    pub quote: AssetId,
    pub sequence: i64,
}

/// Builder for `get_fill_order_history`: the most recent trades (filled orders) on the `base`/`quote`
/// market, newest first.
///
/// `.limit(..)` caps how many fills come back (default 100).
pub struct FillOrderHistoryRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) base: String,
    pub(super) quote: String,
    pub(super) limit: u32,
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

    pub async fn get(self) -> Result<Vec<OrderHistoryObject>, SwaplockApiError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_bitshares_fill_order_history_shape() {
        let json = r#"
        {
            "id": "5.0.199986569",
            "key": { "base": "1.3.0", "quote": "1.3.5589", "sequence": -1919503 },
            "time": "2026-06-28T16:43:57",
            "op": {
                "fee": { "amount": 0, "asset_id": "1.3.0" },
                "order_id": "1.7.572775253",
                "account_id": "1.2.1014025",
                "pays": { "amount": 891353, "asset_id": "1.3.5589" },
                "receives": { "amount": 91914802, "asset_id": "1.3.0" },
                "fill_price": {
                    "base": { "amount": 50000000, "asset_id": "1.3.5589" },
                    "quote": { "amount": "5155914802", "asset_id": "1.3.0" }
                },
                "is_maker": true
            }
        }
        "#;

        let trade: OrderHistoryObject = serde_json::from_str(json).expect("typed order history");

        assert_eq!(trade.id.0, "5.0.199986569");
        assert_eq!(trade.key.base.0, "1.3.0");
        assert_eq!(trade.key.quote.0, "1.3.5589");
        assert_eq!(trade.key.sequence, -1919503);
        assert_eq!(trade.op.order_id.0, "1.7.572775253");
        assert!(trade.op.is_maker);
    }
}
