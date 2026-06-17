use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

/// Builder for `get_block_header`: the header of a produced block, without its transactions.
///
/// Lighter than [`block`](crate::DatabaseApi::block) when you only need the metadata (witness,
/// timestamp, previous id). `Some(header)` once the height is produced, `None` past the head. Raw
/// JSON, for the same reason [`block`](crate::DatabaseApi::block) is.
pub struct GetBlockHeaderRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) block_num: u32,
}

impl GetBlockHeaderRequest<'_> {
    pub async fn get(self) -> Result<Option<Value>, SwaplockApiError> {
        let value = self
            .session
            .database_call("get_block_header", json!([self.block_num]))?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_block_header",
            message: error.to_string(),
        })
    }
}
