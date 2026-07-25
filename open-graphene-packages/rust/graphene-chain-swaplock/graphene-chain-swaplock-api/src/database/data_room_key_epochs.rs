use graphene_chain_swaplock_bindings::generated::DataRoomKeyEpochObject;
use graphene_chain_swaplock_bindings::generated::ids::{DataRoomId, DataRoomKeyEpochId};
use graphene_chain_swaplock_bindings::generated::rpc::database::get_data_room_key_epochs as rpc_get_data_room_key_epochs;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_data_room_key_epochs`: one account's key epoch records in one data room.
///
/// Pass the room id (like `1.9.0`) and the member's account name or id. Page with an optional
/// `.limit(..)` and `.start_id(..)`; key epoch records come back in id order starting from
/// `start_id` (inclusive), and the node applies its own defaults when either is unset.
pub struct DataRoomKeyEpochsRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) room_id: String,
    pub(super) member_name_key_or_id: String,
    pub(super) limit: Option<u32>,
    pub(super) start_id: Option<String>,
}

impl DataRoomKeyEpochsRequest<'_> {
    /// Cap how many key epoch records come back (the node applies its own default and maximum).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Resume paging: only key epoch records with an id at or after `start_id` come back.
    pub fn start_id<S>(mut self, start_id: S) -> Self
    where
        S: Into<String>,
    {
        self.start_id = Some(start_id.into());
        self
    }

    pub async fn get(self) -> Result<Vec<DataRoomKeyEpochObject>, SwaplockApiError> {
        let params = rpc_get_data_room_key_epochs::Params {
            room_id: DataRoomId(self.room_id),
            member_name_key_or_id: self.member_name_key_or_id,
            limit: self.limit,
            start_id: self.start_id.map(DataRoomKeyEpochId),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_key_epochs::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_data_room_key_epochs::METHOD, params)
            .await?;
        rpc_get_data_room_key_epochs::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_key_epochs::METHOD,
        ))
    }
}
