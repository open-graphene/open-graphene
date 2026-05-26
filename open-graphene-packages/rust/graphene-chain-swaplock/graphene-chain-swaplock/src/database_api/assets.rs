use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use crate::rpc::GrapheneRpc;

pub fn lookup_asset_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<String, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    Ok(open_graphene_sdk_live::lookup_asset_id(rpc.session_mut(), symbol)?.to_string())
}

pub fn lookup_asset_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    Ok(
        open_graphene_sdk_live::lookup_asset_id_optional(rpc.session_mut(), symbol)?
            .map(|asset_id| asset_id.to_string()),
    )
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

fn ensure_database_api_id(rpc: &mut GrapheneRpc, api_id: u64) -> Result<(), Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != api_id {
        return Err(format!("database API id mismatch: expected {expected}, got {api_id}").into());
    }
    Ok(())
}
