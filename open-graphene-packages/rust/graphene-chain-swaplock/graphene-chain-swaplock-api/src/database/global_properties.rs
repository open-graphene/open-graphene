use graphene_chain_swaplock_bindings::generated::GlobalPropertyObject;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

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
    let value = session.database_call("get_global_properties", json!([]))?;
    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_global_properties",
        message: error.to_string(),
    })
}
