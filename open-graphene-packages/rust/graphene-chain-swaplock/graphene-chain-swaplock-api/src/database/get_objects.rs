use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

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
        let value = self
            .session
            .database_call("get_objects", json!([self.ids, false]))
            .await?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_objects",
            message: error.to_string(),
        })
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
