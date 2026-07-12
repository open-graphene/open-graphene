use graphene_chain_swaplock_bindings::generated::ContentCardObject;
use graphene_chain_swaplock_bindings::generated::ids::ContentCardId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_content_card_by_id as rpc_get_content_card_by_id;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_content_card_by_id`: fetch one content card by its id (like `1.10.0`).
///
/// You get back `Some(card)` when the card exists, or `None` for an unknown id.
pub struct ContentCardByIdRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) content_id: String,
}

impl ContentCardByIdRequest<'_> {
    pub async fn get(self) -> Result<Option<ContentCardObject>, SwaplockApiError> {
        let params = rpc_get_content_card_by_id::Params {
            content_id: ContentCardId(self.content_id),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_content_card_by_id::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_content_card_by_id::METHOD, params)
            .await?;
        rpc_get_content_card_by_id::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_content_card_by_id::METHOD,
        ))
    }
}
