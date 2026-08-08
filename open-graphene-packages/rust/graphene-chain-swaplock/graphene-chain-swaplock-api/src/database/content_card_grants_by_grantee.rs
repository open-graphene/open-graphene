use graphene_chain_swaplock_bindings::generated::ContentCardGrantObject;
use graphene_chain_swaplock_bindings::generated::ids::ContentCardGrantId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_content_card_grants_by_grantee as rpc_get_content_card_grants_by_grantee;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_content_card_grants_by_grantee`: what has been shared with one recipient.
///
/// The grantee may be an account (name or id) or a bare public key - a process holding only a
/// key uses this to discover every single document it may read. Page with an optional
/// `.limit(..)` and `.start_id(..)`.
pub struct ContentCardGrantsByGranteeRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) grantee_name_key_or_id: String,
    pub(super) limit: Option<u32>,
    pub(super) start_id: Option<String>,
}

impl ContentCardGrantsByGranteeRequest<'_> {
    /// Cap how many grants come back (the node applies its own default and maximum).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Resume paging: only grants with an id at or after `start_id` come back.
    pub fn start_id<S>(mut self, start_id: S) -> Self
    where
        S: Into<String>,
    {
        self.start_id = Some(start_id.into());
        self
    }

    pub async fn get(self) -> Result<Vec<ContentCardGrantObject>, SwaplockApiError> {
        let params = rpc_get_content_card_grants_by_grantee::Params {
            grantee_name_key_or_id: self.grantee_name_key_or_id,
            limit: self.limit,
            start_id: self.start_id.map(ContentCardGrantId),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_content_card_grants_by_grantee::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_content_card_grants_by_grantee::METHOD, params)
            .await?;
        rpc_get_content_card_grants_by_grantee::parse_returns(value).map_err(
            SwaplockApiError::unexpected(rpc_get_content_card_grants_by_grantee::METHOD),
        )
    }
}
