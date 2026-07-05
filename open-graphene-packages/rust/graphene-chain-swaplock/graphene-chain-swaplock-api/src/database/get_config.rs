use graphene_chain_swaplock_bindings::generated::rpc::database::get_config as rpc_get_config;
use open_graphene_transport::GrapheneSession;
use serde_json::Value;

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
        let params = rpc_get_config::Params {}
            .to_params_value()
            .map_err(SwaplockApiError::unexpected(rpc_get_config::METHOD))?;
        let value = self
            .session
            .database_call(rpc_get_config::METHOD, params)
            .await?;
        rpc_get_config::parse_returns(value)
            .map_err(SwaplockApiError::unexpected(rpc_get_config::METHOD))
    }
}
