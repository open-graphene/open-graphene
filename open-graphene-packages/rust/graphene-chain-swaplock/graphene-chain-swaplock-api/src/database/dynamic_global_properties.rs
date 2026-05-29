use graphene_chain_swaplock_bindings::generated::DynamicGlobalPropertyObject;
use open_graphene_transport::{CallbackId, GrapheneSession, JsonRpcInbound};
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::constants::{DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID, DYNAMIC_GLOBAL_PROPERTIES_ID};
use super::objects::find_object_by_id;

pub struct DynamicGlobalPropertiesRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
}

pub struct DynamicGlobalPropertiesSubscription<'session> {
    session: &'session mut GrapheneSession,
    initial: DynamicGlobalPropertyObject,
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
            if callback_id != CallbackId::new(DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID) {
                continue;
            }
            if let Ok(value) = dynamic_global_properties_from_value("notice", payload) {
                return Ok(value);
            }
        }
    }
}

pub(super) async fn get_dynamic_global_properties(
    session: &mut GrapheneSession,
) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
    let value = session.database_call("get_dynamic_global_properties", json!([]))?;
    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_dynamic_global_properties",
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
