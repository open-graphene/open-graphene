use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use serde_json::{json, Value};

use crate::rpc::GrapheneRpc;

pub fn lookup_asset_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<String, Box<dyn Error>> {
    lookup_asset_id_optional(rpc, api_id, symbol)?
        .ok_or_else(|| format!("asset not found: {symbol}").into())
}

pub fn lookup_asset_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let result = rpc.call_database(api_id, "lookup_asset_symbols", json!([[symbol]]))?;
    let Some(asset) = result.as_array().and_then(|values| values.first()) else {
        return Ok(None);
    };
    if asset.is_null() {
        return Ok(None);
    }
    let returned_symbol = asset
        .get("symbol")
        .and_then(Value::as_str)
        .ok_or("lookup_asset_symbols result missing symbol")?;
    if returned_symbol != symbol {
        return Ok(None);
    }
    asset
        .get("id")
        .and_then(Value::as_str)
        .map(|id| Some(id.to_string()))
        .ok_or_else(|| "lookup_asset_symbols result missing id".into())
}

pub fn wait_for_asset(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    for _ in 0..20 {
        if let Some(asset_id) = lookup_asset_id_optional(rpc, api_id, symbol)? {
            return Ok(Some(asset_id));
        }
        sleep(Duration::from_millis(500));
    }
    Ok(None)
}
