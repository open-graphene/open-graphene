use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use serde_json::Value;

use crate::rpc::GrapheneRpc;

pub fn lookup_account_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<String, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    Ok(open_graphene_sdk_live::lookup_account_id(rpc.session_mut(), account_name)?.to_string())
}

pub fn lookup_account_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    Ok(
        open_graphene_sdk_live::lookup_account_id_optional(rpc.session_mut(), account_name)?
            .map(|account_id| account_id.to_string()),
    )
}

pub fn wait_for_account(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    for _ in 0..20 {
        if let Some(account_id) = lookup_account_id_optional(rpc, api_id, account_name)? {
            return Ok(Some(account_id));
        }
        sleep(Duration::from_millis(500));
    }
    Ok(None)
}

pub fn active_public_key_for_account(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let account = rpc
        .get_objects(api_id, [account_id])?
        .get(0)
        .cloned()
        .ok_or("account object was not returned")?;
    let Some(key_auths) = account
        .get("active")
        .and_then(|active| active.get("key_auths"))
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    if key_auths.len() != 1 {
        return Ok(None);
    }
    Ok(key_auths[0]
        .as_array()
        .and_then(|pair| pair.first())
        .and_then(Value::as_str)
        .map(ToOwned::to_owned))
}

fn ensure_database_api_id(rpc: &mut GrapheneRpc, api_id: u64) -> Result<(), Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != api_id {
        return Err(format!("database API id mismatch: expected {expected}, got {api_id}").into());
    }
    Ok(())
}
