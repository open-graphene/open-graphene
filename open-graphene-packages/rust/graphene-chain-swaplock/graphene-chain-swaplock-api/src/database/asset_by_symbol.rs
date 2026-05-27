use graphene_chain_swaplock_bindings::generated::AssetObject;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

use super::asset_by_id::{AssetSubscription, subscribe_asset_by_id};

pub struct AssetBySymbolRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) asset_symbol: String,
}

impl<'session> AssetBySymbolRequest<'session> {
    pub async fn get(self) -> Result<AssetObject, SwaplockApiError> {
        get_asset_by_symbol(self.session, &self.asset_symbol).await
    }

    pub async fn subscribe(self) -> Result<AssetSubscription<'session>, SwaplockApiError> {
        let asset = get_asset_by_symbol(self.session, &self.asset_symbol).await?;
        subscribe_asset_by_id(self.session, asset.id.0).await
    }
}

pub(super) async fn get_asset_by_symbol(
    session: &mut GrapheneSession,
    asset_symbol: &str,
) -> Result<AssetObject, SwaplockApiError> {
    let value = session.database_call("lookup_asset_symbols", json!([[asset_symbol], false]))?;
    let assets = value
        .as_array()
        .ok_or_else(|| SwaplockApiError::UnexpectedResponse {
            method: "lookup_asset_symbols",
            message: "expected asset array".to_string(),
        })?;
    let asset = assets
        .first()
        .and_then(|value| (!value.is_null()).then(|| value.clone()))
        .ok_or_else(|| SwaplockApiError::AssetNotFound {
            asset: asset_symbol.to_string(),
        })?;
    let asset: AssetObject =
        serde_json::from_value(asset).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "lookup_asset_symbols",
            message: error.to_string(),
        })?;
    if asset.symbol != asset_symbol {
        return Err(SwaplockApiError::AssetNotFound {
            asset: asset_symbol.to_string(),
        });
    }

    Ok(asset)
}
