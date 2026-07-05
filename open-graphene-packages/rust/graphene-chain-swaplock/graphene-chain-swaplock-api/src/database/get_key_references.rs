use graphene_chain_swaplock_bindings::generated::ids::AccountId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_key_references as rpc_get_key_references;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::string_list::IntoStringList;

/// Builder for `get_key_references`: which accounts list each public key in their authorities.
///
/// Hand it public keys (the prefixed strings, e.g. `BTS6M…`); you get back one list of
/// [`AccountId`]s per key, in the same order. Handy for wallets working out which accounts a key can
/// act for.
pub struct GetKeyReferencesRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) keys: Vec<String>,
}

impl GetKeyReferencesRequest<'_> {
    pub async fn get(self) -> Result<Vec<Vec<AccountId>>, SwaplockApiError> {
        let params = rpc_get_key_references::Params { keys: self.keys }
            .to_params_value()
            .map_err(SwaplockApiError::unexpected(rpc_get_key_references::METHOD))?;
        let value = self
            .session
            .database_call(rpc_get_key_references::METHOD, params)
            .await?;
        rpc_get_key_references::parse_returns(value)
            .map_err(SwaplockApiError::unexpected(rpc_get_key_references::METHOD))
    }
}

/// Build the request from anything that turns into a list of key strings.
pub(super) fn get_key_references_request<L>(
    session: &mut GrapheneSession,
    keys: L,
) -> GetKeyReferencesRequest<'_>
where
    L: IntoStringList,
{
    GetKeyReferencesRequest {
        session,
        keys: keys.into_string_list(),
    }
}
