use graphene_chain_swaplock_bindings::generated::rpc::database::lookup_accounts as rpc_lookup_accounts;
use open_graphene_transport::GrapheneSession;

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
        let params = rpc_lookup_accounts::Params {
            lower_bound_name: self.lower_bound,
            limit: self.limit,
            subscribe: None,
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(rpc_lookup_accounts::METHOD))?;
        let value = self
            .session
            .database_call(rpc_lookup_accounts::METHOD, params)
            .await?;
        // The node returns a name-to-id map; the generated layer leaves it untyped.
        serde_json::from_value(value)
            .map_err(SwaplockApiError::unexpected(rpc_lookup_accounts::METHOD))
    }
}
