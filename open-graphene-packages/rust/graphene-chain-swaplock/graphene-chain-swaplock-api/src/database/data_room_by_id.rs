use graphene_chain_swaplock_bindings::generated::DataRoomObject;
use graphene_chain_swaplock_bindings::generated::ids::DataRoomId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_data_room_by_id as rpc_get_data_room_by_id;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_data_room_by_id`: fetch one data room by its id (like `1.9.0`).
///
/// You get back `Some(room)` when the room exists, or `None` for an unknown id.
pub struct DataRoomByIdRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) room_id: String,
}

impl DataRoomByIdRequest<'_> {
    pub async fn get(self) -> Result<Option<DataRoomObject>, SwaplockApiError> {
        let params = rpc_get_data_room_by_id::Params {
            room_id: DataRoomId(self.room_id),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_by_id::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_data_room_by_id::METHOD, params)
            .await?;
        rpc_get_data_room_by_id::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_by_id::METHOD,
        ))
    }
}
