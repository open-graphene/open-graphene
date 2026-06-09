use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

use crate::codec::decode;

/// Builder for `orders.get_tracked_groups`: the group widths (in 0.01% units)
/// the node's grouped-orders plugin is configured to track.
pub struct TrackedGroupsRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
}

impl TrackedGroupsRequest<'_> {
    pub async fn get(self) -> Result<Vec<u16>, SwaplockApiError> {
        get_tracked_groups(self.session).await
    }
}

pub(super) async fn get_tracked_groups(
    session: &mut GrapheneSession,
) -> Result<Vec<u16>, SwaplockApiError> {
    let result = session.orders_call("get_tracked_groups", json!([]))?;
    decode("get_tracked_groups", result)
}
