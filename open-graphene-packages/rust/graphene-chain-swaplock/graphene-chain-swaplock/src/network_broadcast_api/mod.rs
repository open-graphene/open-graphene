use std::error::Error;

use serde_json::Value;

use crate::rpc::GrapheneRpc;

pub fn broadcast_transaction(
    rpc: &mut GrapheneRpc,
    network_broadcast_api_id: u64,
    transaction_json: Value,
) -> Result<(), Box<dyn Error>> {
    rpc.broadcast_transaction(network_broadcast_api_id, transaction_json)
}
