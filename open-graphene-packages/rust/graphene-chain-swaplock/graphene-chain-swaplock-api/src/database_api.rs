use graphene_chain_swaplock_bindings::generated::{AccountObject, DynamicGlobalPropertyObject};
use open_graphene_transport::{GrapheneSession, JsonRpcInbound, parse_chain_id};
use serde_json::{Value, json};

use crate::SwaplockApiError;

const DYNAMIC_GLOBAL_PROPERTIES_ID: &str = "2.1.0";
const DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID: u64 = 1;

pub struct DatabaseApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

pub struct DynamicGlobalPropertiesSubscription<'session> {
    session: &'session mut GrapheneSession,
    initial: DynamicGlobalPropertyObject,
}

impl DynamicGlobalPropertiesSubscription<'_> {
    pub fn initial(&self) -> &DynamicGlobalPropertyObject {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice()?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID {
                continue;
            }

            return dynamic_global_properties_from_value("notice", payload);
        }
    }
}

impl<'session> DatabaseApi<'session> {
    pub async fn get_chain_id(&mut self) -> Result<String, SwaplockApiError> {
        let value = self.session.database_call("get_chain_id", json!([]))?;
        Ok(parse_chain_id(value)?)
    }

    pub async fn get_dynamic_global_properties(
        &mut self,
    ) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        let value = self
            .session
            .database_call("get_dynamic_global_properties", json!([]))?;

        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_dynamic_global_properties",
            message: error.to_string(),
        })
    }

    pub async fn subscribe_dynamic_global_properties(
        self,
    ) -> Result<DynamicGlobalPropertiesSubscription<'session>, SwaplockApiError> {
        self.session.database_call(
            "set_subscribe_callback",
            json!([DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID, false]),
        )?;
        let value = self
            .session
            .database_call("get_objects", json!([[DYNAMIC_GLOBAL_PROPERTIES_ID], true]))?;
        let initial = dynamic_global_properties_from_value("get_objects", value)?;

        Ok(DynamicGlobalPropertiesSubscription {
            session: self.session,
            initial,
        })
    }

    pub async fn get_account_by_name(
        &mut self,
        account_name: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        let accounts = self
            .session
            .database_call("lookup_accounts", json!([account_name, 1]))?;
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
        let objects = self
            .session
            .database_call("get_objects", json!([[account_id]]))?;
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

fn dynamic_global_properties_from_value(
    method: &'static str,
    value: Value,
) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
    let value = find_object_by_id(&value, DYNAMIC_GLOBAL_PROPERTIES_ID).ok_or_else(|| {
        SwaplockApiError::UnexpectedResponse {
            method,
            message: format!("missing `{DYNAMIC_GLOBAL_PROPERTIES_ID}` object"),
        }
    })?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}

fn find_object_by_id(value: &Value, id: &str) -> Option<Value> {
    if value.get("id").and_then(Value::as_str) == Some(id) {
        return Some(value.clone());
    }

    value
        .as_array()?
        .iter()
        .find_map(|item| find_object_by_id(item, id))
}
