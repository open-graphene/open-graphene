use std::error::Error;
use std::time::Duration;

use serde_json::{json, Value};

use crate::rpc::GrapheneRpc;

pub fn head_block(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
) -> Result<open_graphene_sdk_core::HeadBlock, Box<dyn Error>> {
    let dgp = rpc
        .call_database(database_api_id, "get_objects", json!([["2.1.0"]]))?
        .get(0)
        .cloned()
        .ok_or("dynamic global properties object was not returned")?;
    Ok(open_graphene_sdk_core::HeadBlock {
        number: dgp
            .get("head_block_number")
            .and_then(Value::as_u64)
            .ok_or("dynamic global properties missing head_block_number")?,
        id: dgp
            .get("head_block_id")
            .and_then(Value::as_str)
            .ok_or("dynamic global properties missing head_block_id")?
            .to_string(),
        time: dgp
            .get("time")
            .and_then(Value::as_str)
            .ok_or("dynamic global properties missing time")?
            .to_string(),
    })
}

pub fn next_transaction_header(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    expiration: Duration,
) -> Result<open_graphene_sdk_core::TransactionHeader, Box<dyn Error>> {
    open_graphene_sdk_core::transaction_header_from_head(
        &head_block(rpc, database_api_id)?,
        expiration,
    )
    .map_err(Into::into)
}
