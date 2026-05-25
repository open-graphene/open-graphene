use std::error::Error;

use serde_json::{json, Value};

use crate::rpc::GrapheneRpc;

pub fn broadcast_transaction(
    rpc: &mut GrapheneRpc,
    network_broadcast_api_id: u64,
    transaction_json: Value,
) -> Result<(), Box<dyn Error>> {
    rpc.call_network_broadcast(
        network_broadcast_api_id,
        "broadcast_transaction",
        json!([transaction_json]),
    )?;
    Ok(())
}
