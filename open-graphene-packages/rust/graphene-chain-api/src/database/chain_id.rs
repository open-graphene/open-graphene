use bindings::generated::rpc::database::get_chain_id as rpc_get_chain_id;
use open_graphene_transport::GrapheneSession;

use crate::ChainApiError;

pub struct ChainIdRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
}

impl ChainIdRequest<'_> {
    pub async fn get(self) -> Result<String, ChainApiError> {
        get_chain_id(self.session).await
    }
}

pub(super) async fn get_chain_id(
    session: &mut GrapheneSession,
) -> Result<String, ChainApiError> {
    let params = rpc_get_chain_id::Params {}
        .to_params_value()
        .map_err(ChainApiError::unexpected(rpc_get_chain_id::METHOD))?;
    let value = session
        .database_call(rpc_get_chain_id::METHOD, params)
        .await?;
    rpc_get_chain_id::parse_returns(value)
        .map_err(ChainApiError::unexpected(rpc_get_chain_id::METHOD))
}
