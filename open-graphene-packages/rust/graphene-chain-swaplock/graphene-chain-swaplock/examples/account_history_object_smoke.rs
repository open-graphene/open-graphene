use std::env;
use std::error::Error;

use graphene_chain_swaplock::{
    database_api::lookup_account_id,
    history_api::{get_account_history_objects, AccountHistoryQuery},
    rpc::GrapheneRpc,
};

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = env::var("GRAPHENE_RPC_URL")
        .unwrap_or_else(|_| "wss://node02.swaplock.chainpool.online:8090".to_string());
    let account_name =
        env::var("GRAPHENE_ACCOUNT_NAME").unwrap_or_else(|_| "committee-account".to_string());

    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;
    let history_api_id = rpc.history_api_id()?;
    let account_id = lookup_account_id(&mut rpc, database_api_id, &account_name)?;
    let query = AccountHistoryQuery::recent(&account_id);
    let history = get_account_history_objects(&mut rpc, history_api_id, &query)?;

    println!("Connected: database_api_id={database_api_id} history_api_id={history_api_id}");
    println!("Account: {account_name} -> {account_id}");
    println!("History objects: {}", history.len());

    if let Some(entry) = history.first() {
        println!(
            "Latest history object: id={} block_num={} op_in_trx={} virtual_op={} block_time={}",
            entry.id.0, entry.block_num, entry.op_in_trx, entry.virtual_op, entry.block_time
        );
    }

    Ok(())
}
