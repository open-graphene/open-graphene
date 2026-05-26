use graphene_chain_swaplock_bindings::generated::{
    AccountObject, ChainPropertyObject, DynamicGlobalPropertyObject, GlobalPropertyObject,
};
use open_graphene_transport::{GrapheneSession, JsonRpcInbound, parse_chain_id};
use serde_json::{Value, json};

use crate::SwaplockApiError;

const ACCOUNT_CALLBACK_ID: u64 = 2;
const DYNAMIC_GLOBAL_PROPERTIES_ID: &str = "2.1.0";
const DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID: u64 = 1;

pub struct DatabaseApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

pub struct AccountByIdRequest<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
}

pub struct AccountByNameRequest<'session> {
    session: &'session mut GrapheneSession,
    account_name: String,
}

pub struct AccountSubscription<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    initial: AccountObject,
}

pub struct AccountsRequest<'session> {
    session: &'session mut GrapheneSession,
    names_or_ids: Vec<String>,
}

pub struct ChainIdRequest<'session> {
    session: &'session mut GrapheneSession,
}

pub struct ChainPropertiesRequest<'session> {
    session: &'session mut GrapheneSession,
}

pub struct DynamicGlobalPropertiesRequest<'session> {
    session: &'session mut GrapheneSession,
}

pub struct DynamicGlobalPropertiesSubscription<'session> {
    session: &'session mut GrapheneSession,
    initial: DynamicGlobalPropertyObject,
}

pub struct GlobalPropertiesRequest<'session> {
    session: &'session mut GrapheneSession,
}

impl<'session> AccountByIdRequest<'session> {
    pub async fn get(self) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_id(self.session, &self.account_id).await
    }

    pub async fn subscribe(self) -> Result<AccountSubscription<'session>, SwaplockApiError> {
        subscribe_account_by_id(self.session, self.account_id).await
    }
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

impl AccountSubscription<'_> {
    pub fn initial(&self) -> &AccountObject {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<AccountObject, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice()?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != ACCOUNT_CALLBACK_ID {
                continue;
            }

            return account_from_value("notice", payload, &self.account_id);
        }
    }
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
                message: "expected array response".to_string(),
            })?;

        accounts
            .iter()
            .map(|account| {
                if account.is_null() {
                    Ok(None)
                } else {
                    serde_json::from_value(account.clone())
                        .map(Some)
                        .map_err(|error| SwaplockApiError::UnexpectedResponse {
                            method: "get_accounts",
                            message: error.to_string(),
                        })
                }
            })
            .collect()
    }
}

impl ChainIdRequest<'_> {
    pub async fn get(self) -> Result<String, SwaplockApiError> {
        get_chain_id(self.session).await
    }
}

impl ChainPropertiesRequest<'_> {
    pub async fn get(self) -> Result<ChainPropertyObject, SwaplockApiError> {
        get_chain_properties(self.session).await
    }
}

impl<'session> DynamicGlobalPropertiesRequest<'session> {
    pub async fn get(self) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        get_dynamic_global_properties(self.session).await
    }

    pub async fn subscribe(
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

impl GlobalPropertiesRequest<'_> {
    pub async fn get(self) -> Result<GlobalPropertyObject, SwaplockApiError> {
        get_global_properties(self.session).await
    }
}

impl<'session> DatabaseApi<'session> {
    pub fn account_by_id<S>(self, account_id: S) -> AccountByIdRequest<'session>
    where
        S: Into<String>,
    {
        AccountByIdRequest {
            session: self.session,
            account_id: account_id.into(),
        }
    }

    pub fn account_by_name<S>(self, account_name: S) -> AccountByNameRequest<'session>
    where
        S: Into<String>,
    {
        AccountByNameRequest {
            session: self.session,
            account_name: account_name.into(),
        }
    }

    pub fn accounts<I, S>(self, names_or_ids: I) -> AccountsRequest<'session>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        AccountsRequest {
            session: self.session,
            names_or_ids: names_or_ids.into_iter().map(Into::into).collect(),
        }
    }

    pub fn chain_id(self) -> ChainIdRequest<'session> {
        ChainIdRequest {
            session: self.session,
        }
    }

    pub fn chain_properties(self) -> ChainPropertiesRequest<'session> {
        ChainPropertiesRequest {
            session: self.session,
        }
    }

    pub fn dynamic_global_properties(self) -> DynamicGlobalPropertiesRequest<'session> {
        DynamicGlobalPropertiesRequest {
            session: self.session,
        }
    }

    pub fn global_properties(self) -> GlobalPropertiesRequest<'session> {
        GlobalPropertiesRequest {
            session: self.session,
        }
    }

    pub async fn get_account_by_name(
        &mut self,
        account_name: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_name(self.session, account_name).await
    }

    pub async fn get_account_by_id(
        &mut self,
        account_id: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_id(self.session, account_id).await
    }

    pub async fn get_chain_id(&mut self) -> Result<String, SwaplockApiError> {
        get_chain_id(self.session).await
    }

    pub async fn get_chain_properties(&mut self) -> Result<ChainPropertyObject, SwaplockApiError> {
        get_chain_properties(self.session).await
    }

    pub async fn get_dynamic_global_properties(
        &mut self,
    ) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        get_dynamic_global_properties(self.session).await
    }

    pub async fn get_global_properties(
        &mut self,
    ) -> Result<GlobalPropertyObject, SwaplockApiError> {
        get_global_properties(self.session).await
    }

    pub async fn subscribe_dynamic_global_properties(
        self,
    ) -> Result<DynamicGlobalPropertiesSubscription<'session>, SwaplockApiError> {
        self.dynamic_global_properties().subscribe().await
    }
}

async fn get_account_by_name(
    session: &mut GrapheneSession,
    account_name: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let accounts = session.database_call("lookup_accounts", json!([account_name, 1]))?;
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

    get_account_by_id(session, &account_id).await
}

async fn get_account_by_id(
    session: &mut GrapheneSession,
    account_id: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let objects = session.database_call("get_objects", json!([[account_id]]))?;
    account_from_value("get_objects", objects, account_id)
}

async fn get_chain_id(session: &mut GrapheneSession) -> Result<String, SwaplockApiError> {
    let value = session.database_call("get_chain_id", json!([]))?;
    Ok(parse_chain_id(value)?)
}

async fn get_chain_properties(
    session: &mut GrapheneSession,
) -> Result<ChainPropertyObject, SwaplockApiError> {
    let value = session.database_call("get_chain_properties", json!([]))?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_chain_properties",
        message: error.to_string(),
    })
}

async fn get_dynamic_global_properties(
    session: &mut GrapheneSession,
) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
    let value = session.database_call("get_dynamic_global_properties", json!([]))?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_dynamic_global_properties",
        message: error.to_string(),
    })
}

async fn get_global_properties(
    session: &mut GrapheneSession,
) -> Result<GlobalPropertyObject, SwaplockApiError> {
    let value = session.database_call("get_global_properties", json!([]))?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_global_properties",
        message: error.to_string(),
    })
}

async fn subscribe_account_by_id(
    session: &mut GrapheneSession,
    account_id: String,
) -> Result<AccountSubscription<'_>, SwaplockApiError> {
    session.database_call(
        "set_subscribe_callback",
        json!([ACCOUNT_CALLBACK_ID, false]),
    )?;
    let value = session.database_call("get_objects", json!([[account_id], true]))?;
    let initial = account_from_value("get_objects", value, &account_id)?;

    Ok(AccountSubscription {
        session,
        account_id,
        initial,
    })
}

fn account_from_value(
    method: &'static str,
    value: Value,
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
