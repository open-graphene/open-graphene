use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::{SignedTransfer, SwaplockApiError};

pub struct NetworkBroadcastApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BroadcastReceipt {
    transaction_json: Value,
}

impl<'session> NetworkBroadcastApi<'session> {
    pub async fn broadcast_signed_transfer(
        self,
        signed: SignedTransfer,
    ) -> Result<BroadcastReceipt, SwaplockApiError> {
        let transaction_json = signed.transaction_json()?;
        self.broadcast_transaction_json(transaction_json).await
    }

    pub async fn broadcast_transaction_json(
        self,
        transaction_json: Value,
    ) -> Result<BroadcastReceipt, SwaplockApiError> {
        self.session
            .network_broadcast_call("broadcast_transaction", json!([transaction_json.clone()]))?;
        Ok(BroadcastReceipt { transaction_json })
    }
}

impl BroadcastReceipt {
    pub fn transaction_json(&self) -> &Value {
        &self.transaction_json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_preserves_submitted_transaction_json() {
        let transaction_json = json!({
            "ref_block_num": 1,
            "operations": [],
            "signatures": ["abc"]
        });
        let receipt = BroadcastReceipt {
            transaction_json: transaction_json.clone(),
        };

        assert_eq!(receipt.transaction_json(), &transaction_json);
    }
}
