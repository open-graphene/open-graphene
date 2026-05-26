use std::env;
use std::error::Error;

use graphene_chain_swaplock::{
    database_api::{account_balance, lookup_account_id, lookup_asset_id},
    rpc::GrapheneRpc,
};

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = env::var("GRAPHENE_RPC_URL")
        .unwrap_or_else(|_| "wss://node02.swaplock.chainpool.online:8090".to_string());
    let account_name = env::var("GRAPHENE_ACCOUNT_NAME").unwrap_or_else(|_| "swaplock".to_string());
    let asset_symbol = env::var("GRAPHENE_ASSET_SYMBOL").unwrap_or_else(|_| "BTS".to_string());

    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;
    let account_id = lookup_account_id(&mut rpc, database_api_id, &account_name)?;
    let asset_id = lookup_asset_id(&mut rpc, database_api_id, &asset_symbol)?;
    let balance = account_balance(&mut rpc, database_api_id, &account_id, &asset_id)?;

    println!("Connected: database_api_id={database_api_id}");
    println!("Account: {account_name} -> {account_id}");
    println!("Asset: {asset_symbol} -> {asset_id}");
    println!("Balance: {balance} {asset_id}");

    Ok(())
}
