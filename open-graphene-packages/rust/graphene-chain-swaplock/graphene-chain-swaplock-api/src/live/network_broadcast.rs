use std::time::Duration;

use open_graphene_transport::{LiveTransportHandle, PendingCallbackNotice};
use serde_json::json;

use crate::network_broadcast::parse_broadcast_confirmation;
use crate::{BroadcastConfirmation, SignedTransfer, SwaplockApiError};

pub struct SwaplockLiveNetworkBroadcastApi {
    pub(crate) live: LiveTransportHandle,
    pub(crate) network_broadcast_api_id: u64,
}

pub struct SwaplockLivePendingBroadcastConfirmation {
    pending: PendingCallbackNotice,
}

impl SwaplockLiveNetworkBroadcastApi {
    pub async fn send_signed_transfer_with_callback(
        self,
        signed: SignedTransfer,
    ) -> Result<SwaplockLivePendingBroadcastConfirmation, SwaplockApiError> {
        self.send_transaction_with_callback(signed.transaction_json()?)
            .await
    }

    pub async fn send_transaction_with_callback(
        self,
        transaction: serde_json::Value,
    ) -> Result<SwaplockLivePendingBroadcastConfirmation, SwaplockApiError> {
        let pending = self
            .live
            .call_with_callback(
                self.network_broadcast_api_id,
                "broadcast_transaction_with_callback",
                json!([transaction]),
            )
            .await?;
        Ok(SwaplockLivePendingBroadcastConfirmation { pending })
    }
}

impl SwaplockLivePendingBroadcastConfirmation {
    pub async fn wait(self) -> Result<BroadcastConfirmation, SwaplockApiError> {
        parse_live_broadcast_confirmation(self.pending.wait().await?)
    }

    pub async fn wait_timeout(
        self,
        timeout: Duration,
    ) -> Result<BroadcastConfirmation, SwaplockApiError> {
        parse_live_broadcast_confirmation(self.pending.wait_timeout(timeout).await?)
    }
}

fn parse_live_broadcast_confirmation(
    value: serde_json::Value,
) -> Result<BroadcastConfirmation, SwaplockApiError> {
    parse_broadcast_confirmation(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn live_confirmation_parser_accepts_single_item_callback_payload() {
        let confirmation = parse_live_broadcast_confirmation(json!([{
            "id": {"_hash": [1, 2, 3, 4, 5]},
            "block_num": 123,
            "trx_num": 4,
            "trx": {
                "ref_block_num": 1,
                "ref_block_prefix": 2,
                "expiration": "2026-01-01T00:00:00",
                "operations": [],
                "extensions": [],
                "signatures": ["abcdef"],
                "operation_results": []
            }
        }]))
        .unwrap();

        assert_eq!(confirmation.block_num(), 123);
        assert_eq!(confirmation.trx_num(), 4);
        assert_eq!(confirmation.id(), &json!({"_hash": [1, 2, 3, 4, 5]}));
        assert_eq!(confirmation.transaction()["signatures"], json!(["abcdef"]));
    }

    #[test]
    fn live_confirmation_parser_reports_malformed_payload() {
        let error = parse_live_broadcast_confirmation(json!({"block_num": 123})).unwrap_err();
        assert!(
            matches!(error, SwaplockApiError::UnexpectedResponse { method, .. } if method == "network_broadcast.broadcast_transaction_with_callback")
        );
    }
}
