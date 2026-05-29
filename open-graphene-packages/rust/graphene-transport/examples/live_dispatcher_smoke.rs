use std::error::Error;
use std::time::Duration;

use open_graphene_transport::{CallbackId, GrapheneSession};
use serde_json::json;

const DYNAMIC_GLOBAL_PROPERTIES_ID: &str = "2.1.0";
const DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID: CallbackId = CallbackId::new(42_001);
const WAIT_TIMEOUT: Duration = Duration::from_secs(15);

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = std::env::var("SWAPLOCK_RPC_URL")
        .unwrap_or_else(|_| "wss://node02.swaplock.chainpool.online:8090".to_string());
    let session = GrapheneSession::connect(&rpc_url)?;
    let database_api_id = session.api_ids().database;
    let chain_id = session.chain_id().to_string();
    let live = session.into_live_transport()?;

    println!("connected chain_id={chain_id}");
    println!("database_api_id={database_api_id}");

    let subscription = live.subscribe_callback(DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID)?;
    live.call(
        database_api_id,
        "set_subscribe_callback",
        json!([DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID.as_u64(), false]),
    )?
    .wait_timeout(WAIT_TIMEOUT)?;

    let initial = live
        .call(
            database_api_id,
            "get_objects",
            json!([[DYNAMIC_GLOBAL_PROPERTIES_ID], true]),
        )?
        .wait_timeout(WAIT_TIMEOUT)?;
    println!("subscribed dynamic_global_properties initial={initial}");

    let mut pending = Vec::new();
    for index in 1..=5 {
        let response = live.call(database_api_id, "get_dynamic_global_properties", json!([]))?;
        println!("sent get_dynamic_global_properties #{index}");
        pending.push((index, response));
    }

    for (index, response) in pending {
        let value = response.wait_timeout(WAIT_TIMEOUT)?;
        println!(
            "response #{index}: head_block={}",
            value["head_block_number"].as_u64().unwrap_or_default()
        );
    }

    let notice = subscription.next_timeout(WAIT_TIMEOUT)?;
    println!("subscription notice={notice}");

    Ok(())
}
