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

#[derive(Clone, Debug, PartialEq)]
pub struct TransferBroadcastReceipt {
    transaction_json: Value,
    from_id: String,
    to_id: String,
    asset_id: String,
    amount: i64,
    fee_amount: i64,
    fee_asset_id: String,
    min_block_num: u64,
}

impl<'session> NetworkBroadcastApi<'session> {
    pub async fn broadcast_signed_transfer(
        self,
        signed: SignedTransfer,
    ) -> Result<TransferBroadcastReceipt, SwaplockApiError> {
        let transaction_json = signed.transaction_json()?;
        let receipt = TransferBroadcastReceipt {
            transaction_json: transaction_json.clone(),
            from_id: signed.from_id().to_string(),
            to_id: signed.to_id().to_string(),
            asset_id: signed.asset_id().to_string(),
            amount: signed.amount(),
            fee_amount: signed.fee().amount,
            fee_asset_id: signed.fee().asset_id.0.clone(),
            min_block_num: signed.head_block_number(),
        };
        self.session
            .network_broadcast_call("broadcast_transaction", json!([transaction_json]))?;
        Ok(receipt)
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

impl TransferBroadcastReceipt {
    pub fn transaction_json(&self) -> &Value {
        &self.transaction_json
    }

    #[cfg(test)]
    pub(crate) fn from_parts_for_tests(
        transaction_json: Value,
        from_id: String,
        to_id: String,
        asset_id: String,
        amount: i64,
        fee_amount: i64,
        fee_asset_id: String,
        min_block_num: u64,
    ) -> Self {
        Self {
            transaction_json,
            from_id,
            to_id,
            asset_id,
            amount,
            fee_amount,
            fee_asset_id,
            min_block_num,
        }
    }

    pub fn from_id(&self) -> &str {
        &self.from_id
    }

    pub fn to_id(&self) -> &str {
        &self.to_id
    }

    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    pub fn amount(&self) -> i64 {
        self.amount
    }

    pub fn fee_amount(&self) -> i64 {
        self.fee_amount
    }

    pub fn fee_asset_id(&self) -> &str {
        &self.fee_asset_id
    }

    pub fn min_block_num(&self) -> u64 {
        self.min_block_num
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

    #[test]
    fn transfer_receipt_preserves_confirmation_fields() {
        let receipt = TransferBroadcastReceipt {
            transaction_json: json!({"operations": []}),
            from_id: "1.2.100".to_string(),
            to_id: "1.2.0".to_string(),
            asset_id: "1.3.0".to_string(),
            amount: 1,
            fee_amount: 200_000,
            fee_asset_id: "1.3.0".to_string(),
            min_block_num: 123,
        };

        assert_eq!(receipt.from_id(), "1.2.100");
        assert_eq!(receipt.to_id(), "1.2.0");
        assert_eq!(receipt.asset_id(), "1.3.0");
        assert_eq!(receipt.amount(), 1);
        assert_eq!(receipt.fee_amount(), 200_000);
        assert_eq!(receipt.fee_asset_id(), "1.3.0");
        assert_eq!(receipt.min_block_num(), 123);
    }
}
