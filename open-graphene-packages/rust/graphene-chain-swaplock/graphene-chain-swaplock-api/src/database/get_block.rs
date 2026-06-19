use graphene_chain_swaplock_bindings::generated::types::SignedBlock;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

/// Builder for `get_block`: fetch a produced block by its height.
///
/// You get back `Some(block)` once the height has been produced, or `None` for a height the chain
/// has not reached yet. The block is the typed [`SignedBlock`] (witness, timestamp, transactions,
/// witness signature, merkle root).
pub struct GetBlockRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) block_num: u32,
}

impl GetBlockRequest<'_> {
    pub async fn get(self) -> Result<Option<SignedBlock>, SwaplockApiError> {
        let value = self
            .session
            .database_call("get_block", json!([self.block_num]))
            .await?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_block",
            message: error.to_string(),
        })
    }
}
