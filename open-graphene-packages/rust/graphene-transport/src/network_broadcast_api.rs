use serde_json::{Value, json};

use crate::{GrapheneSession, TransportError};

pub fn broadcast_transaction(
    session: &mut GrapheneSession,
    transaction_json: Value,
) -> Result<(), TransportError> {
    session.network_broadcast_call(
        "broadcast_transaction",
        broadcast_transaction_params(transaction_json),
    )?;
    Ok(())
}

fn broadcast_transaction_params(transaction_json: Value) -> Value {
    json!([transaction_json])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_transaction_json_in_graphene_params_array() {
        let transaction = json!({
            "ref_block_num": 1,
            "operations": [],
            "signatures": ["abc"]
        });

        assert_eq!(
            broadcast_transaction_params(transaction.clone()),
            json!([transaction])
        );
    }
}
