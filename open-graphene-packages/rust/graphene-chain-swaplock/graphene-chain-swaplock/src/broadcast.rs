use std::error::Error;

use graphene_chain_swaplock_bindings::generated::types::{SignedTransaction, Transaction};
use serde_json::Value;

use crate::network_broadcast_api::broadcast_transaction;
use crate::rpc::GrapheneRpc;
use crate::signing::sign_transaction_checked;

pub fn sign_and_broadcast_transaction<F, E>(
    rpc: &mut GrapheneRpc,
    network_broadcast_api_id: u64,
    transaction: &Transaction,
    wif: &str,
    expected_public_key: &str,
    renderer: F,
) -> Result<SignedTransaction, Box<dyn Error>>
where
    F: Fn(&SignedTransaction) -> Result<Value, E>,
    E: Error + 'static,
{
    let signed_transaction = sign_transaction_checked(transaction, wif, expected_public_key)?;
    broadcast_transaction(
        rpc,
        network_broadcast_api_id,
        renderer(&signed_transaction).map_err(|err| -> Box<dyn Error> { Box::new(err) })?,
    )?;
    Ok(signed_transaction)
}
