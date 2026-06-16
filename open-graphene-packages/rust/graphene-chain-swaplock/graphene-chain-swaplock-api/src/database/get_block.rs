use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

/// Builder for `get_block`: fetch a produced block by its height.
///
/// You get back `Some(block)` once the height has been produced, or `None` for a height the chain
/// has not reached yet. The block comes back as raw JSON (witness, timestamp, transactions, the
/// `*_with_info` extras); deserialize the parts you need.
//
// Raw JSON rather than the generated `SignedBlock`: that type deserializes a `Signature` from a
// byte sequence, but the node sends the witness signature as a hex string, so it would not parse.
pub struct GetBlockRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) block_num: u32,
}

impl GetBlockRequest<'_> {
    pub async fn get(self) -> Result<Option<Value>, SwaplockApiError> {
        let value = self
            .session
            .database_call("get_block", json!([self.block_num]))?;
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_block",
            message: error.to_string(),
        })
    }
}
