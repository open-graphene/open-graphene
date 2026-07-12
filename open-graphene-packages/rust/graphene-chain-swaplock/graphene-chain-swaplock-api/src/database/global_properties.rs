use graphene_chain_swaplock_bindings::generated::GlobalPropertyObject;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_global_properties as rpc_get_global_properties;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

pub struct GlobalPropertiesRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
}

impl GlobalPropertiesRequest<'_> {
    pub async fn get(self) -> Result<GlobalPropertyObject, SwaplockApiError> {
        get_global_properties(self.session).await
    }
}

pub(super) async fn get_global_properties(
    session: &mut GrapheneSession,
) -> Result<GlobalPropertyObject, SwaplockApiError> {
    let params = rpc_get_global_properties::Params {}
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_global_properties::METHOD,
        ))?;
    let value = session
        .database_call(rpc_get_global_properties::METHOD, params)
        .await?;
    rpc_get_global_properties::parse_returns(value).map_err(SwaplockApiError::unexpected(
        rpc_get_global_properties::METHOD,
    ))
}
