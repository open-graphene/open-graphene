use graphene_chain_swaplock_bindings::generated::ChainPropertyObject;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_chain_properties as rpc_get_chain_properties;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

pub struct ChainPropertiesRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
}

impl ChainPropertiesRequest<'_> {
    pub async fn get(self) -> Result<ChainPropertyObject, SwaplockApiError> {
        get_chain_properties(self.session).await
    }
}

pub(super) async fn get_chain_properties(
    session: &mut GrapheneSession,
) -> Result<ChainPropertyObject, SwaplockApiError> {
    let params = rpc_get_chain_properties::Params {}
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_chain_properties::METHOD,
        ))?;
    let value = session
        .database_call(rpc_get_chain_properties::METHOD, params)
        .await?;
    rpc_get_chain_properties::parse_returns(value).map_err(SwaplockApiError::unexpected(
        rpc_get_chain_properties::METHOD,
    ))
}
