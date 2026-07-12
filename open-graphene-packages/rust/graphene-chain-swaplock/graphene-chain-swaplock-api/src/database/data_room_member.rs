use graphene_chain_swaplock_bindings::generated::DataRoomMemberObject;
use graphene_chain_swaplock_bindings::generated::ids::DataRoomId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_data_room_member as rpc_get_data_room_member;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_data_room_member`: one account's member record in one data room.
///
/// Pass the room id (like `1.9.0`) and the member's account name or id. You get back
/// `Some(member)` when the account is a member of the room, or `None` when it is not.
pub struct DataRoomMemberRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) room_id: String,
    pub(super) account_name_or_id: String,
}

impl DataRoomMemberRequest<'_> {
    pub async fn get(self) -> Result<Option<DataRoomMemberObject>, SwaplockApiError> {
        let params = rpc_get_data_room_member::Params {
            room_id: DataRoomId(self.room_id),
            account_name_or_id: self.account_name_or_id,
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_member::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_data_room_member::METHOD, params)
            .await?;
        rpc_get_data_room_member::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_data_room_member::METHOD,
        ))
    }
}
