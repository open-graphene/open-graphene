use graphene_chain_swaplock_bindings::generated::AssetObject;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

/// Default page size when the caller does not set one.
pub const DEFAULT_LIST_ASSETS_LIMIT: u32 = 100;

/// Builder for `list_assets`: walk the chain's assets in symbol order.
///
/// Pass the symbol to start from (`""` for the beginning) and an optional `.limit(..)`. Useful for
/// asset discovery, e.g. to find what markets a chain has.
pub struct ListAssetsRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) lower_bound: String,
    pub(super) limit: u32,
}

impl ListAssetsRequest<'_> {
    /// Cap how many assets come back (node has its own maximum, usually 100).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub async fn get(self) -> Result<Vec<AssetObject>, SwaplockApiError> {
        let value = self
            .session
            .database_call("list_assets", json!([self.lower_bound, self.limit]))?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "list_assets",
            message: error.to_string(),
        })
    }
}
