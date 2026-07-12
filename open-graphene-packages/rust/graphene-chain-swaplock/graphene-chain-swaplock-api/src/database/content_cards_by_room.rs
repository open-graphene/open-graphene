use graphene_chain_swaplock_bindings::generated::ContentCardObject;
use graphene_chain_swaplock_bindings::generated::ids::{ContentCardId, DataRoomId};
use graphene_chain_swaplock_bindings::generated::rpc::database::get_content_cards_by_room as rpc_get_content_cards_by_room;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_content_cards_by_room`: the content cards of one data room.
///
/// Pass the room id (like `1.9.0`). Page with an optional `.limit(..)` and `.start_id(..)`;
/// cards come back in id order starting from `start_id` (inclusive), and the node applies its
/// own defaults when either is unset.
pub struct ContentCardsByRoomRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) room_id: String,
    pub(super) limit: Option<u32>,
    pub(super) start_id: Option<String>,
}

impl ContentCardsByRoomRequest<'_> {
    /// Cap how many cards come back (the node applies its own default and maximum).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Resume paging: only cards with an id at or after `start_id` come back.
    pub fn start_id<S>(mut self, start_id: S) -> Self
    where
        S: Into<String>,
    {
        self.start_id = Some(start_id.into());
        self
    }

    pub async fn get(self) -> Result<Vec<ContentCardObject>, SwaplockApiError> {
        let params = rpc_get_content_cards_by_room::Params {
            room_id: DataRoomId(self.room_id),
            limit: self.limit,
            start_id: self.start_id.map(ContentCardId),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_content_cards_by_room::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_content_cards_by_room::METHOD, params)
            .await?;
        rpc_get_content_cards_by_room::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_content_cards_by_room::METHOD,
        ))
    }
}
