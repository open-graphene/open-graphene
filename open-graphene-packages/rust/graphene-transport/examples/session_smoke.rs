use std::error::Error;

use open_graphene_transport::GrapheneSession;

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = std::env::var("SWAPLOCK_RPC_URL")?;
    let session = GrapheneSession::connect(&rpc_url)?;

    println!("chain_id={}", session.chain_id());
    println!("database_api_id={}", session.api_ids().database);
    match session.api_ids().history {
        Some(api_id) => println!("history_api_id={api_id}"),
        None => println!("history_api_id=<unavailable>"),
    }
    match session.api_ids().network_broadcast {
        Some(api_id) => println!("network_broadcast_api_id={api_id}"),
        None => println!("network_broadcast_api_id=<unavailable>"),
    }

    Ok(())
}
