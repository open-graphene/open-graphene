use graphene_chain_swaplock_bindings::generated::AccountObject;
use open_graphene_transport::{GrapheneSession, get_objects, lookup_accounts, parse_chain_id};
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

    pub async fn get_account_by_name(
        &mut self,
        account_name: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        let accounts = lookup_accounts(self.session, account_name, 1)?;
        let account_id = accounts
            .as_array()
            .and_then(|rows| {
                rows.iter().find_map(|row| {
                    let row = row.as_array()?;
                    let name = row.first()?.as_str()?;
                    let id = row.get(1)?.as_str()?;
                    (name == account_name).then(|| id.to_string())
                })
            })
            .ok_or_else(|| SwaplockApiError::AccountNotFound {
                account: account_name.to_string(),
            })?;

        self.get_account_by_id(&account_id).await
    }

    pub async fn get_account_by_id(
        &mut self,
        account_id: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        let objects = get_objects(self.session, [account_id])?;
        let account = objects
            .as_array()
            .and_then(|objects| objects.first())
            .filter(|object| !object.is_null())
            .cloned()
            .ok_or_else(|| SwaplockApiError::AccountNotFound {
                account: account_id.to_string(),
            })?;

        serde_json::from_value(account).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_objects",
            message: error.to_string(),
        })
    }
}
