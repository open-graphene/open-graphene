use graphene_chain_swaplock_bindings::generated::AssetObject;
use open_graphene_transport::{CallbackId, GrapheneSession, JsonRpcInbound};
use serde_json::json;

use crate::SwaplockApiError;

use super::constants::ASSET_CALLBACK_ID;
use super::objects::find_object_by_id;

pub struct AssetByIdRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) asset_id: String,
}

pub struct AssetSubscription<'session> {
    session: &'session mut GrapheneSession,
    asset_id: String,
    initial: AssetObject,
}

impl<'session> AssetByIdRequest<'session> {
    pub async fn get(self) -> Result<AssetObject, SwaplockApiError> {
        get_asset_by_id(self.session, &self.asset_id).await
    }

    pub async fn subscribe(self) -> Result<AssetSubscription<'session>, SwaplockApiError> {
        subscribe_asset_by_id(self.session, self.asset_id).await
    }
}

impl AssetSubscription<'_> {
    pub fn initial(&self) -> &AssetObject {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<AssetObject, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice().await?;
            let JsonRpcInbound::Notice {
                callback_id,
                payload,
            } = notice
            else {
                continue;
            };
            if callback_id != CallbackId::new(ASSET_CALLBACK_ID) {
                continue;
            }

            if let Ok(asset) = asset_from_value("notice", payload, &self.asset_id) {
                return Ok(asset);
            }
        }
    }
}

pub(super) async fn get_asset_by_id(
    session: &mut GrapheneSession,
    asset_id: &str,
) -> Result<AssetObject, SwaplockApiError> {
    let value = session
        .database_call("get_assets", json!([[asset_id], false]))
        .await?;
    let assets = value
        .as_array()
        .ok_or_else(|| SwaplockApiError::UnexpectedResponse {
            method: "get_assets",
            message: "expected asset array".to_string(),
        })?;
    let asset = assets
        .first()
        .and_then(|value| (!value.is_null()).then(|| value.clone()))
        .ok_or_else(|| SwaplockApiError::AssetNotFound {
            asset: asset_id.to_string(),
        })?;

    serde_json::from_value(asset).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_assets",
        message: error.to_string(),
    })
}

pub(super) async fn subscribe_asset_by_id(
    session: &mut GrapheneSession,
    asset_id: String,
) -> Result<AssetSubscription<'_>, SwaplockApiError> {
    session
        .database_call("set_subscribe_callback", json!([ASSET_CALLBACK_ID, false]))
        .await?;
    let value = session
        .database_call("get_objects", json!([[asset_id], true]))
        .await?;
    let initial = asset_from_value("get_objects", value, &asset_id)?;

    Ok(AssetSubscription {
        session,
        asset_id,
        initial,
    })
}

pub(crate) fn asset_from_value(
    method: &'static str,
    value: serde_json::Value,
    asset_id: &str,
) -> Result<AssetObject, SwaplockApiError> {
    let value = find_object_by_id(&value, asset_id).ok_or_else(|| {
        SwaplockApiError::UnexpectedResponse {
            method,
            message: format!("missing `{asset_id}` asset object"),
        }
    })?;

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}
