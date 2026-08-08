use graphene_chain_swaplock_bindings::generated::ContentCardGrantObject;
use graphene_chain_swaplock_bindings::generated::ids::ContentCardId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_content_card_grant as rpc_get_content_card_grant;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_content_card_grant`: one grant, addressed by card and grantee.
///
/// The grantee may be an account (name or id) or a bare public key. Returns `None` when the
/// card is not granted to that grantee.
pub struct ContentCardGrantRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) content_id: String,
    pub(super) grantee_name_key_or_id: String,
}

impl ContentCardGrantRequest<'_> {
    pub async fn get(self) -> Result<Option<ContentCardGrantObject>, SwaplockApiError> {
        let params = rpc_get_content_card_grant::Params {
            content_id: ContentCardId(self.content_id),
            grantee_name_key_or_id: self.grantee_name_key_or_id,
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_content_card_grant::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_content_card_grant::METHOD, params)
            .await?;
        rpc_get_content_card_grant::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_content_card_grant::METHOD,
        ))
    }
}
