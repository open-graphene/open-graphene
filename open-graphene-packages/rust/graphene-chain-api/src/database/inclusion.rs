//! Finding a transaction in the blocks after the one it was prepared at, by its id, without
//! sending it again: what to do after a broadcast whose outcome is unknown.

use std::time::{Duration, Instant};

use bindings::generated::types::{ProcessedTransaction, Transaction};
use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::{ChainApiError, transaction_id};

use super::dynamic_global_properties::get_dynamic_global_properties;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Where a transaction was included, with its operation results.
#[derive(Clone, Debug, PartialEq)]
pub struct IncludedTransaction {
    pub id: String,
    pub block_num: u32,
    /// Its position among the block's transactions.
    pub index: u32,
    pub block_time: String,
    pub transaction: ProcessedTransaction,
}

pub(super) async fn find_transaction_in_block(
    session: &mut GrapheneSession,
    block_num: u32,
    id: &str,
) -> Result<Option<IncludedTransaction>, ChainApiError> {
    let block = session
        .database_call("get_block", json!([block_num]))
        .await?;
    if block.is_null() {
        return Err(ChainApiError::UnexpectedResponse {
            method: "get_block",
            message: format!("the node has no block {block_num}"),
        });
    }
    let block_time = block
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let transactions = block
        .get("transactions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (index, candidate) in transactions.into_iter().enumerate() {
        // Other transactions may carry what these bindings cannot decode; they are not ours.
        let Ok(transaction) = serde_json::from_value::<ProcessedTransaction>(candidate) else {
            continue;
        };
        let unsigned = Transaction {
            ref_block_num: transaction.ref_block_num,
            ref_block_prefix: transaction.ref_block_prefix,
            expiration: transaction.expiration.clone(),
            operations: transaction.operations.clone(),
            extensions: transaction.extensions.clone(),
        };
        if transaction_id(&unsigned).is_ok_and(|candidate| candidate == id) {
            return Ok(Some(IncludedTransaction {
                id: id.to_string(),
                block_num,
                index: index as u32,
                block_time,
                transaction,
            }));
        }
    }
    Ok(None)
}

pub(super) async fn wait_for_inclusion(
    session: &mut GrapheneSession,
    id: &str,
    after_block: u32,
    timeout: Duration,
) -> Result<IncludedTransaction, ChainApiError> {
    let deadline = Instant::now() + timeout;
    let mut next = after_block + 1;
    loop {
        let head = get_dynamic_global_properties(session)
            .await?
            .head_block_number;
        while next <= head {
            if let Some(found) = find_transaction_in_block(session, next, id).await? {
                return Ok(found);
            }
            next += 1;
        }
        if Instant::now() + POLL_INTERVAL > deadline {
            return Err(ChainApiError::InclusionNotObserved { id: id.to_string() });
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
