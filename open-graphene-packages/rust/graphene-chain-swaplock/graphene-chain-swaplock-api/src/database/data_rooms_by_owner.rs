use graphene_chain_swaplock_bindings::generated::DataRoomObject;
use graphene_chain_swaplock_bindings::generated::ids::DataRoomId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_data_rooms_by_owner as rpc_get_data_rooms_by_owner;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_data_rooms_by_owner`: the data rooms owned by an account.
///
/// Pass the owner's account name or id. Page with an optional `.limit(..)` and `.start_id(..)`;
/// rooms come back in id order starting from `start_id` (inclusive), and the node applies its own
/// defaults when either is unset.
pub struct DataRoomsByOwnerRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_name_or_id: String,
    pub(super) limit: Option<u32>,
    pub(super) start_id: Option<String>,
}

impl DataRoomsByOwnerRequest<'_> {
    /// Cap how many rooms come back (the node applies its own default and maximum).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Resume paging: only rooms with an id at or after `start_id` come back.
    pub fn start_id<S>(mut self, start_id: S) -> Self
    where
        S: Into<String>,
    {
        self.start_id = Some(start_id.into());
        self
    }

    pub async fn get(self) -> Result<Vec<DataRoomObject>, SwaplockApiError> {
        let params = rpc_get_data_rooms_by_owner::Params {
            account_name_or_id: self.account_name_or_id,
            limit: self.limit,
            start_id: self.start_id.map(DataRoomId),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_data_rooms_by_owner::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_data_rooms_by_owner::METHOD, params)
            .await?;
        rpc_get_data_rooms_by_owner::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_data_rooms_by_owner::METHOD,
        ))
    }
}
