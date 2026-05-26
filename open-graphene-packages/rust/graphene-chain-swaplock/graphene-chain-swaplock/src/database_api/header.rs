use std::error::Error;
use std::time::Duration;

use crate::rpc::GrapheneRpc;

pub fn head_block(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
) -> Result<open_graphene_sdk_core::HeadBlock, Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != database_api_id {
        return Err(format!(
            "database API id mismatch: expected {expected}, got {database_api_id}"
        )
        .into());
    }
    Ok(open_graphene_sdk_live::head_block(rpc.session_mut())?)
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
