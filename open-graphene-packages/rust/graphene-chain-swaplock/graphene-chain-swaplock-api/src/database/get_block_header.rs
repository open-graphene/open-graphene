use graphene_chain_swaplock_bindings::generated::types::MaybeSignedBlockHeader;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

/// Builder for `get_block_header`: the header of a produced block, without its transactions.
///
/// Lighter than [`block`](crate::DatabaseApi::block) when you only need the metadata (witness,
/// timestamp, previous id). `Some(header)` once the height is produced, `None` past the head. The
/// header is the typed [`MaybeSignedBlockHeader`] (the witness signature is optional).
pub struct GetBlockHeaderRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) block_num: u32,
}

impl GetBlockHeaderRequest<'_> {
    pub async fn get(self) -> Result<Option<MaybeSignedBlockHeader>, SwaplockApiError> {
        let value = self
            .session
            .database_call("get_block_header", json!([self.block_num]))
            .await?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_block_header",
            message: error.to_string(),
        })
    }
}
