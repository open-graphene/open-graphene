use std::env;
use std::error::Error;

use graphene_chain_swaplock::{database_api::head_block, rpc::GrapheneRpc};

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = env::var("GRAPHENE_RPC_URL")
        .unwrap_or_else(|_| "wss://node02.swaplock.chainpool.online:8090".to_string());
    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;
    let head = head_block(&mut rpc, database_api_id)?;

    println!("Connected: database_api_id={database_api_id}");
    println!("Head block: {} {} {}", head.number, head.id, head.time);

    Ok(())
}
