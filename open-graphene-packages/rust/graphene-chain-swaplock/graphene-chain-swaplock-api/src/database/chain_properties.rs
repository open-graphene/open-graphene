use graphene_chain_swaplock_bindings::generated::ChainPropertyObject;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

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
    let value = session
        .database_call("get_chain_properties", json!([]))
        .await?;
    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_chain_properties",
        message: error.to_string(),
    })
}
