use graphene_chain_swaplock_bindings::generated::ContentCardObject;
use graphene_chain_swaplock_bindings::generated::ids::ContentCardId;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_content_cards_by_author as rpc_get_content_cards_by_author;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_content_cards_by_author`: the content cards authored by an account.
///
/// Pass the author's account name or id. Page with an optional `.limit(..)` and `.start_id(..)`;
/// cards come back in id order starting from `start_id` (inclusive), and the node applies its
/// own defaults when either is unset.
pub struct ContentCardsByAuthorRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_name_or_id: String,
    pub(super) limit: Option<u32>,
    pub(super) start_id: Option<String>,
}

impl ContentCardsByAuthorRequest<'_> {
    /// Cap how many cards come back (the node applies its own default and maximum).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Resume paging: only cards with an id at or after `start_id` come back.
    pub fn start_id<S>(mut self, start_id: S) -> Self
    where
        S: Into<String>,
    {
        self.start_id = Some(start_id.into());
        self
    }

    pub async fn get(self) -> Result<Vec<ContentCardObject>, SwaplockApiError> {
        let params = rpc_get_content_cards_by_author::Params {
            account_name_or_id: self.account_name_or_id,
            limit: self.limit,
            start_id: self.start_id.map(ContentCardId),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(
            rpc_get_content_cards_by_author::METHOD,
        ))?;
        let value = self
            .session
            .database_call(rpc_get_content_cards_by_author::METHOD, params)
            .await?;
        rpc_get_content_cards_by_author::parse_returns(value).map_err(SwaplockApiError::unexpected(
            rpc_get_content_cards_by_author::METHOD,
        ))
    }
}
