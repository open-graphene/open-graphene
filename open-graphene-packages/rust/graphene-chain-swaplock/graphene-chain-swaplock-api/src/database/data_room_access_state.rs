use graphene_chain_swaplock_bindings::generated::ids::DataRoomId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_data_room_access_state as rpc_get_data_room_access_state;
use graphene_chain_swaplock_bindings::generated::types::DataRoomAccessState;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_data_room_access_state`: read one coherent access snapshot (like `1.23.0`).
///
/// You get back `Some(state)` when the room exists, or `None` for an unknown id.
pub struct DataRoomAccessStateRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) room_id: String,
}

impl DataRoomAccessStateRequest<'_> {
    pub async fn get(self) -> Result<Option<DataRoomAccessState>, SwaplockApiError> {
        open_graphene_core::ObjectId::parse(&self.room_id)?.require_type(1, 23)?;
        let expected_room = self.room_id.clone();
        let params = rpc_get_data_room_access_state::Params {
            room_id: DataRoomId(self.room_id),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_access_state::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_data_room_access_state::METHOD, params)
            .await?;
        let state = rpc_get_data_room_access_state::parse_returns(value).map_err(
            SwaplockApiError::unexpected(rpc_get_data_room_access_state::METHOD),
        )?;
        if let Some(snapshot) = &state {
            let condition = crate::RoomAccessPrecondition::from_snapshot(snapshot)?;
            if condition.room() != expected_room {
                return Err(SwaplockApiError::InvalidRoomAccess {
                    message: "Snapshot belongs to another room".into(),
                });
            }
        }
        Ok(state)
    }
}
