use std::time::Duration;

use open_graphene_transport::{GrapheneSession, PendingCallback};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{SignedTransfer, SwaplockApiError};

pub struct NetworkBroadcastApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BroadcastReceipt {
    transaction: Value,
}

pub struct PendingBroadcastConfirmation {
    pending: PendingCallback,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BroadcastConfirmation {
    pub id: Value,
    pub block_num: u32,
    pub trx_num: u32,
    pub trx: Value,
}

impl<'session> NetworkBroadcastApi<'session> {
    pub async fn broadcast_signed_transfer(
        self,
        signed: SignedTransfer,
    ) -> Result<BroadcastReceipt, SwaplockApiError> {
        self.broadcast_transaction(signed.transaction_json()?).await
    }

    pub async fn broadcast_signed_transfer_with_callback(
        self,
        signed: SignedTransfer,
    ) -> Result<BroadcastConfirmation, SwaplockApiError> {
        self.broadcast_transaction_with_callback(signed.transaction_json()?)
            .await
    }

    pub async fn broadcast_signed_transfer_with_callback_timeout(
        self,
        signed: SignedTransfer,
        timeout: Duration,
    ) -> Result<BroadcastConfirmation, SwaplockApiError> {
        self.broadcast_transaction_with_callback_timeout(signed.transaction_json()?, timeout)
            .await
    }

    pub async fn send_signed_transfer_with_callback(
        self,
        signed: SignedTransfer,
    ) -> Result<PendingBroadcastConfirmation, SwaplockApiError> {
        self.send_transaction_with_callback(signed.transaction_json()?)
            .await
    }

    pub async fn broadcast_transaction(
        self,
        transaction: Value,
    ) -> Result<BroadcastReceipt, SwaplockApiError> {
        self.session
            .network_broadcast_call("broadcast_transaction", json!([transaction.clone()]))
            .await?;
        Ok(BroadcastReceipt { transaction })
    }

    pub async fn broadcast_transaction_with_callback(
        self,
        transaction: Value,
    ) -> Result<BroadcastConfirmation, SwaplockApiError> {
        let confirmation = self
            .session
            .network_broadcast_call_with_callback(
                "broadcast_transaction_with_callback",
                json!([transaction]),
            )
            .await?;
        parse_broadcast_confirmation(confirmation)
    }

    pub async fn broadcast_transaction_with_callback_timeout(
        self,
        transaction: Value,
        timeout: Duration,
    ) -> Result<BroadcastConfirmation, SwaplockApiError> {
        let confirmation = self
            .session
            .network_broadcast_call_with_callback_timeout(
                "broadcast_transaction_with_callback",
                json!([transaction]),
                timeout,
            )
            .await?;
        parse_broadcast_confirmation(confirmation)
    }

    pub async fn send_transaction_with_callback(
        self,
        transaction: Value,
    ) -> Result<PendingBroadcastConfirmation, SwaplockApiError> {
        let pending = self
            .session
            .network_broadcast_send_callback_request(
                "broadcast_transaction_with_callback",
                json!([transaction]),
            )
            .await?;
        Ok(PendingBroadcastConfirmation { pending })
    }

    pub async fn wait_for_broadcast_confirmation_timeout(
        self,
        pending: PendingBroadcastConfirmation,
        timeout: Duration,
    ) -> Result<BroadcastConfirmation, SwaplockApiError> {
        let confirmation = self
            .session
            .network_broadcast_wait_callback_response_and_notice_timeout(pending.pending, timeout)
            .await?;
        parse_broadcast_confirmation(confirmation)
    }
}

impl BroadcastReceipt {
    pub fn transaction(&self) -> &Value {
        &self.transaction
    }
}

impl BroadcastConfirmation {
    pub fn id(&self) -> &Value {
        &self.id
    }

    pub fn block_num(&self) -> u32 {
        self.block_num
    }

    pub fn trx_num(&self) -> u32 {
        self.trx_num
    }

    pub fn transaction(&self) -> &Value {
        &self.trx
    }
}

pub(crate) fn parse_broadcast_confirmation(
    mut value: Value,
) -> Result<BroadcastConfirmation, SwaplockApiError> {
    if let Some(values) = value.as_array_mut()
        && values.len() == 1
    {
        value = values.remove(0);
    }

    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "network_broadcast.broadcast_transaction_with_callback",
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_preserves_submitted_transaction_json() {
        let transaction = json!({
            "ref_block_num": 1,
            "operations": [],
            "signatures": ["abc"]
        });
        let receipt = BroadcastReceipt {
            transaction: transaction.clone(),
        };

        assert_eq!(receipt.transaction(), &transaction);
    }

    #[test]
    fn parses_broadcast_confirmation() {
        let confirmation = parse_broadcast_confirmation(json!({
            "id": {"_hash": [1, 2, 3, 4, 5]},
            "block_num": 123,
            "trx_num": 4,
            "trx": {
                "ref_block_num": 1,
                "ref_block_prefix": 2,
                "expiration": "2026-01-01T00:00:00",
                "operations": [],
                "extensions": [],
                "signatures": [],
                "operation_results": []
            }
        }))
        .unwrap();

        assert_eq!(confirmation.id(), &json!({"_hash": [1, 2, 3, 4, 5]}));
        assert_eq!(confirmation.block_num(), 123);
        assert_eq!(confirmation.trx_num(), 4);
        assert_eq!(
            confirmation.transaction()["operations"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn parses_single_item_callback_payload() {
        let confirmation = parse_broadcast_confirmation(json!([{
            "id": {"_hash": [1, 2, 3, 4, 5]},
            "block_num": 123,
            "trx_num": 4,
            "trx": {
                "ref_block_num": 1,
                "ref_block_prefix": 2,
                "expiration": "2026-01-01T00:00:00",
                "operations": [],
                "extensions": [],
                "signatures": [],
                "operation_results": []
            }
        }]))
        .unwrap();

        assert_eq!(confirmation.block_num(), 123);
    }
}
