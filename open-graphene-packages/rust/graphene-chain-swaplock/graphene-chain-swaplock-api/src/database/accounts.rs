use graphene_chain_swaplock_bindings::generated::AccountObject;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

pub struct AccountsRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) names_or_ids: Vec<String>,
}

impl AccountsRequest<'_> {
    pub async fn get(self) -> Result<Vec<Option<AccountObject>>, SwaplockApiError> {
        let value = self
            .session
            .database_call("get_accounts", json!([self.names_or_ids, false]))?;
        let accounts = value
            .as_array()
            .ok_or_else(|| SwaplockApiError::UnexpectedResponse {
                method: "get_accounts",
                message: "expected account array".to_string(),
            })?;

        accounts
            .iter()
            .map(|value| {
                if value.is_null() {
                    return Ok(None);
                }
                serde_json::from_value(value.clone())
                    .map(Some)
                    .map_err(|error| SwaplockApiError::UnexpectedResponse {
                        method: "get_accounts",
                        message: error.to_string(),
                    })
            })
            .collect()
    }
}
