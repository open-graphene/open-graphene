use open_graphene_transport::{GrapheneSession, parse_chain_id};
use serde_json::json;

use crate::SwaplockApiError;

pub struct DatabaseApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl DatabaseApi<'_> {
    pub async fn get_chain_id(&mut self) -> Result<String, SwaplockApiError> {
        let value = self.session.database_call("get_chain_id", json!([]))?;
        Ok(parse_chain_id(value)?)
    }
}
