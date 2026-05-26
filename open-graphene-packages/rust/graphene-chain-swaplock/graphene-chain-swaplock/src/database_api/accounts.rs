use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use serde_json::{json, Value};

use crate::rpc::GrapheneRpc;

pub fn lookup_account_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<String, Box<dyn Error>> {
    lookup_account_id_optional(rpc, api_id, account_name)?
        .ok_or_else(|| format!("account not found: {account_name}").into())
}

pub fn lookup_account_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let result = rpc.call_database(api_id, "lookup_accounts", json!([account_name, 1]))?;
    let Some(pair) = result.as_array().and_then(|values| values.first()) else {
        return Ok(None);
    };
    let returned_name = pair
        .get(0)
        .and_then(Value::as_str)
        .ok_or("lookup_accounts result missing account name")?;
    if returned_name != account_name {
        return Ok(None);
    }
    pair.get(1)
        .and_then(Value::as_str)
        .map(|id| Some(id.to_string()))
        .ok_or_else(|| "lookup_accounts result missing account id".into())
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
