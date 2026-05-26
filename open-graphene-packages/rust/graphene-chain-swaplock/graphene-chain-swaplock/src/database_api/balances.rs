use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use open_graphene_sdk_core::{AccountIdRef, AssetIdRef};

use crate::rpc::GrapheneRpc;

pub fn account_balance(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
    asset_id: &str,
) -> Result<i64, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    let account_id = AccountIdRef::parse(account_id)?;
    let asset_id = AssetIdRef::parse(asset_id)?;
    Ok(open_graphene_sdk_live::account_balance(rpc.session_mut(), &account_id, &asset_id)?.amount)
}

pub fn wait_for_balance_at_least(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
    asset_id: &str,
    minimum: i64,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..20 {
        let balance = account_balance(rpc, api_id, account_id, asset_id)?;
        if balance >= minimum {
            println!("Balance ok: {account_id} has {balance} {asset_id}");
            return Ok(());
        }
        sleep(Duration::from_millis(500));
    }
    Err(format!("balance for {account_id} {asset_id} did not reach {minimum}").into())
}

fn ensure_database_api_id(rpc: &mut GrapheneRpc, api_id: u64) -> Result<(), Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != api_id {
        return Err(format!("database API id mismatch: expected {expected}, got {api_id}").into());
    }
    Ok(())
}
