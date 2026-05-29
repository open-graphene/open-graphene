use serde_json::{Value, json};

use crate::{GrapheneSession, TransportError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountHistoryQuery {
    pub account_id: String,
    pub stop: String,
    pub limit: u64,
    pub start: String,
}

impl AccountHistoryQuery {
    pub fn new(
        account_id: impl Into<String>,
        stop: impl Into<String>,
        limit: u64,
        start: impl Into<String>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            stop: stop.into(),
            limit,
            start: start.into(),
        }
    }

    pub fn recent(account_id: impl Into<String>) -> Self {
        Self::recent_with_limit(account_id, 20)
    }

    pub fn recent_with_limit(account_id: impl Into<String>, limit: u64) -> Self {
        Self::new(account_id, "1.11.0", limit, "1.11.0")
    }

    pub fn params(&self) -> Value {
        json!([self.account_id, self.stop, self.limit, self.start])
    }
}

pub fn get_account_history(
    session: &mut GrapheneSession,
    query: &AccountHistoryQuery,
) -> Result<Value, TransportError> {
    session.history_call("get_account_history", query.params())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_account_history_query_uses_existing_wire_shape() {
        assert_eq!(
            AccountHistoryQuery::recent("1.2.100").params(),
            json!(["1.2.100", "1.11.0", 20, "1.11.0"])
        );
    }

    #[test]
    fn recent_account_history_query_allows_custom_limit() {
        assert_eq!(
            AccountHistoryQuery::recent_with_limit("1.2.100", 5).params(),
            json!(["1.2.100", "1.11.0", 5, "1.11.0"])
        );
    }

    #[test]
    fn custom_account_history_query_preserves_wire_shape() {
        assert_eq!(
            AccountHistoryQuery::new("1.2.100", "1.11.5", 3, "1.11.9").params(),
            json!(["1.2.100", "1.11.5", 3, "1.11.9"])
        );
    }
}
