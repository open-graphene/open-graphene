use graphene_chain_swaplock_bindings::generated::ids::AccountId;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

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
        let value = self
            .session
            .database_call("get_key_references", json!([self.keys]))
            .await?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_key_references",
            message: error.to_string(),
        })
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
