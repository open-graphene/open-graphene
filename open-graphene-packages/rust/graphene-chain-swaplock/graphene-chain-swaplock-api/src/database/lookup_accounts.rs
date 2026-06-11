use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

/// Default page size when the caller does not set one.
pub const DEFAULT_LOOKUP_ACCOUNTS_LIMIT: u32 = 100;

/// Builder for `lookup_accounts`: account name to id pairs in name order.
///
/// Good for autocomplete: pass what the user typed as the lower bound and an optional `.limit(..)`.
/// Returns `(name, id)` pairs, e.g. `("swaplock", "1.2.100")`.
pub struct LookupAccountsRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) lower_bound: String,
    pub(super) limit: u32,
}

impl LookupAccountsRequest<'_> {
    /// Cap how many names come back (node has its own maximum, usually 1000).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub async fn get(self) -> Result<Vec<(String, String)>, SwaplockApiError> {
        let value = self
            .session
            .database_call("lookup_accounts", json!([self.lower_bound, self.limit]))?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "lookup_accounts",
            message: error.to_string(),
        })
    }
}
