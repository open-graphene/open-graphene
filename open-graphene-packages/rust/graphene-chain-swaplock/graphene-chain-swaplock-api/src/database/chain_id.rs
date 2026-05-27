use open_graphene_transport::{GrapheneSession, parse_chain_id};
use serde_json::json;

use crate::SwaplockApiError;

pub struct ChainIdRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
}

impl ChainIdRequest<'_> {
    pub async fn get(self) -> Result<String, SwaplockApiError> {
        get_chain_id(self.session).await
    }
}

pub(super) async fn get_chain_id(
    session: &mut GrapheneSession,
) -> Result<String, SwaplockApiError> {
    let value = session.database_call("get_chain_id", json!([]))?;
    parse_chain_id(value).map_err(Into::into)
}
