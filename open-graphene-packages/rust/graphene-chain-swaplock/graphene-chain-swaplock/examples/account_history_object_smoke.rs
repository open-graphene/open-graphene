use std::env;
use std::error::Error;

use graphene_chain_swaplock::{
    database_api::lookup_account_id,
    history_api::{get_account_history_objects, AccountHistoryQuery},
    SwaplockSession,
};

fn main() -> Result<(), Box<dyn Error>> {
    let account_name =
        env::var("GRAPHENE_ACCOUNT_NAME").unwrap_or_else(|_| "committee-account".to_string());

    let mut session = SwaplockSession::connect_from_env_or_default()?;
    let database_api_id = session.database_api_id();
    let history_api_id = session.history_api_id()?;
    let account_id = lookup_account_id(session.rpc_mut(), database_api_id, &account_name)?;
    let query = AccountHistoryQuery::recent(&account_id);
    let history = get_account_history_objects(session.rpc_mut(), history_api_id, &query)?;

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
