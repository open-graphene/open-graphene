use graphene_chain_swaplock_bindings::generated::ids::ObjectId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_objects as rpc_get_objects;
use open_graphene_transport::GrapheneSession;
use serde_json::Value;

use crate::SwaplockApiError;

use super::string_list::IntoStringList;

/// Builder for `get_objects`: fetch any chain objects by id in one round trip.
///
/// This is the generic escape hatch behind the typed getters (`account_by_id`, `asset_by_id`, …).
/// Reach for it when you want an object the SDK has no typed wrapper for yet, or a mixed bag of
/// ids at once. You get back raw JSON in the same order as the ids; an id the node does not know
/// comes back as `null`, so check before you deserialize.
pub struct GetObjectsRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) ids: Vec<String>,
}

impl GetObjectsRequest<'_> {
    pub async fn get(self) -> Result<Vec<Value>, SwaplockApiError> {
        let params = rpc_get_objects::Params {
            ids: self.ids.into_iter().map(ObjectId::from).collect(),
            subscribe: Some(false),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(rpc_get_objects::METHOD))?;
        let value = self
            .session
            .database_call(rpc_get_objects::METHOD, params)
            .await?;
        let objects = rpc_get_objects::parse_returns(value)
            .map_err(SwaplockApiError::unexpected(rpc_get_objects::METHOD))?;
        // Keep the id order intact: unknown ids stay in place as JSON `null`.
        Ok(objects
            .into_iter()
            .map(|object| object.unwrap_or(Value::Null))
            .collect())
    }
}

/// Build the request from anything that turns into a list of id strings.
pub(super) fn get_objects_request<L>(session: &mut GrapheneSession, ids: L) -> GetObjectsRequest<'_>
where
    L: IntoStringList,
{
    GetObjectsRequest {
        session,
        ids: ids.into_string_list(),
    }
}
