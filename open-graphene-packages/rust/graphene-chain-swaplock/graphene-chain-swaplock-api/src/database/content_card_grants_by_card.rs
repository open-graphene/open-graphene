use graphene_chain_swaplock_bindings::generated::ContentCardGrantObject;
use graphene_chain_swaplock_bindings::generated::ids::{ContentCardGrantId, ContentCardId};
use graphene_chain_swaplock_bindings::generated::rpc::database::get_content_card_grants_by_card as rpc_get_content_card_grants_by_card;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_content_card_grants_by_card`: the grants of one content card.
///
/// Pass the card id (like `1.26.7`). Page with an optional `.limit(..)` and `.start_id(..)`;
/// grants come back in id order starting from `start_id` (inclusive), and the node applies its
/// own defaults when either is unset.
pub struct ContentCardGrantsByCardRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) content_id: String,
    pub(super) limit: Option<u32>,
    pub(super) start_id: Option<String>,
}

impl ContentCardGrantsByCardRequest<'_> {
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
        let params = rpc_get_content_card_grants_by_card::Params {
            content_id: ContentCardId(self.content_id),
            limit: self.limit,
            start_id: self.start_id.map(ContentCardGrantId),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_content_card_grants_by_card::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_content_card_grants_by_card::METHOD, params)
            .await?;
        rpc_get_content_card_grants_by_card::parse_returns(value).map_err(
            SwaplockApiError::unexpected(rpc_get_content_card_grants_by_card::METHOD),
        )
    }
}
