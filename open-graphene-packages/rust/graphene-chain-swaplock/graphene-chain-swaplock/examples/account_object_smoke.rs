use std::env;
use std::error::Error;

use graphene_chain_swaplock::{
    database_api::{account_object, lookup_account_id},
    rpc::GrapheneRpc,
};

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = env::var("GRAPHENE_RPC_URL")
        .unwrap_or_else(|_| "wss://node02.swaplock.chainpool.online:8090".to_string());
    let account_name =
        env::var("GRAPHENE_ACCOUNT_NAME").unwrap_or_else(|_| "committee-account".to_string());

    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;
    let account_id = lookup_account_id(&mut rpc, database_api_id, &account_name)?;
    let account = account_object(&mut rpc, database_api_id, &account_id)?
        .ok_or_else(|| format!("account object {account_id} was not found"))?;

    println!("Connected: database_api_id={database_api_id}");
    println!("Account: {account_name} -> {account_id}");
    println!(
        "Account object: id={} name={} active_keys={} owner_keys={}",
        account.id.0,
        account.name,
        account.active.key_auths.len(),
        account.owner.key_auths.len()
    );

    Ok(())
}
