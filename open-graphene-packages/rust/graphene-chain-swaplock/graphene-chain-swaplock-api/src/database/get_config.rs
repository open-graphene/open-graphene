use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

/// Builder for `get_config`: the chain's compile-time constants (the `GRAPHENE_*` parameters).
///
/// These are fixed for a chain (symbol, max sizes, fee scale, intervals, …), so a node returns the
/// same map every time. Comes back as raw JSON; read the keys you need.
pub struct GetConfigRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
}

impl GetConfigRequest<'_> {
    pub async fn get(self) -> Result<Value, SwaplockApiError> {
        let value = self.session.database_call("get_config", json!([]))?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_config",
            message: error.to_string(),
        })
    }
}
