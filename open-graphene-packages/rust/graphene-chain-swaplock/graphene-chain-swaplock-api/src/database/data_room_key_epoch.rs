use graphene_chain_swaplock_bindings::generated::DataRoomKeyEpochObject;
use graphene_chain_swaplock_bindings::generated::ids::DataRoomId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_data_room_key_epoch as rpc_get_data_room_key_epoch;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_data_room_key_epoch`: one account's key record for one epoch of a data room.
///
/// Pass the room id (like `1.9.0`), the key epoch number, and the member's account name or id.
/// You get back `Some(record)` when the record exists, or `None` when it does not.
pub struct DataRoomKeyEpochRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) room_id: String,
    pub(super) epoch: u32,
    pub(super) member_name_key_or_id: String,
}

impl DataRoomKeyEpochRequest<'_> {
    pub async fn get(self) -> Result<Option<DataRoomKeyEpochObject>, SwaplockApiError> {
        let params = rpc_get_data_room_key_epoch::Params {
            room_id: DataRoomId(self.room_id),
            epoch: self.epoch,
            member_name_key_or_id: self.member_name_key_or_id,
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_key_epoch::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_data_room_key_epoch::METHOD, params)
            .await?;
        rpc_get_data_room_key_epoch::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_key_epoch::METHOD,
        ))
    }
}
