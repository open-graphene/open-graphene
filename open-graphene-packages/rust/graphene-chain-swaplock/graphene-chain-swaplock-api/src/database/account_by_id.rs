use graphene_chain_swaplock_bindings::generated::AccountObject;
use open_graphene_transport::{CallbackId, GrapheneSession, JsonRpcInbound};
use serde_json::json;

use crate::SwaplockApiError;

use super::constants::ACCOUNT_CALLBACK_ID;
use super::objects::find_object_by_id;

pub struct AccountByIdRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_id: String,
}

pub struct AccountSubscription<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    initial: AccountObject,
}

impl<'session> AccountByIdRequest<'session> {
    pub async fn get(self) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_id(self.session, &self.account_id).await
    }

    pub async fn subscribe(self) -> Result<AccountSubscription<'session>, SwaplockApiError> {
        subscribe_account_by_id(self.session, self.account_id).await
    }
}

impl AccountSubscription<'_> {
    pub fn initial(&self) -> &AccountObject {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<AccountObject, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice().await?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != CallbackId::new(ACCOUNT_CALLBACK_ID) {
                continue;
            }

            if let Ok(account) = account_from_value("notice", payload, &self.account_id) {
                return Ok(account);
            }
        }
    }
}

pub(super) async fn get_account_by_id(
    session: &mut GrapheneSession,
    account_id: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let value = session
        .database_call("get_accounts", json!([[account_id], false]))
        .await?;
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
            account: account_id.to_string(),
        })?;

    serde_json::from_value(account).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_accounts",
        message: error.to_string(),
    })
}

pub(super) async fn subscribe_account_by_id(
    session: &mut GrapheneSession,
    account_id: String,
) -> Result<AccountSubscription<'_>, SwaplockApiError> {
    session
        .database_call(
            "set_subscribe_callback",
            json!([ACCOUNT_CALLBACK_ID, false]),
        )
        .await?;
    let value = session
        .database_call("get_objects", json!([[account_id], true]))
        .await?;
    let initial = account_from_value("get_objects", value, &account_id)?;

    Ok(AccountSubscription {
        session,
        account_id,
        initial,
    })
}

pub(crate) fn account_from_value(
    method: &'static str,
    value: serde_json::Value,
    account_id: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let value = find_object_by_id(&value, account_id).ok_or_else(|| {
        SwaplockApiError::UnexpectedResponse {
            method,
            message: format!("missing `{account_id}` account object"),
        }
    })?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}
