use graphene_chain_swaplock_bindings::generated::AccountObject;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

use super::account_by_id::{AccountSubscription, subscribe_account_by_id};

pub struct AccountByNameRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_name: String,
}

impl<'session> AccountByNameRequest<'session> {
    pub async fn get(self) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_name(self.session, &self.account_name).await
    }

    pub async fn subscribe(self) -> Result<AccountSubscription<'session>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        subscribe_account_by_id(self.session, account.id.0).await
    }
}

pub(super) async fn get_account_by_name(
    session: &mut GrapheneSession,
    account_name: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let value = session.database_call("get_accounts", json!([[account_name], false]))?;
    let accounts = value
        .as_array()
        .ok_or_else(|| SwaplockApiError::UnexpectedResponse {
            method: "get_accounts",
            message: "expected account array".to_string(),
        })?;
    let account = accounts
        .first()
        .and_then(|value| (!value.is_null()).then(|| value.clone()))
        .ok_or_else(|| SwaplockApiError::AccountNotFound {
            account: account_name.to_string(),
        })?;

    serde_json::from_value(account).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_accounts",
        message: error.to_string(),
    })
}
