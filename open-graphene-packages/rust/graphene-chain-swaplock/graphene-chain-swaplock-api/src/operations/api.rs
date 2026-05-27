use std::thread;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::OperationHistoryObject;
use open_graphene_transport::GrapheneSession;

use crate::{HistoryApi, SwaplockApiError, TransferBroadcastReceipt};

use super::transfer::{PreparedTransfer, SignedTransfer, TransferRequest};

const DEFAULT_CONFIRMATION_ATTEMPTS: u32 = 10;
const DEFAULT_CONFIRMATION_DELAY: Duration = Duration::from_secs(2);
const DEFAULT_CONFIRMATION_LIMIT: u32 = 20;

pub struct OperationsApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferConfirmation {
    history: OperationHistoryObject,
}

impl<'session> OperationsApi<'session> {
    pub fn transfer(self) -> TransferRequest<'session> {
        TransferRequest::new(self.session)
    }

    pub async fn sign_transfer_with_wif(
        self,
        prepared: PreparedTransfer,
        wif: &str,
    ) -> Result<SignedTransfer, SwaplockApiError> {
        let mut database = crate::DatabaseApi {
            session: self.session,
        };
        let account = database.get_account_by_id(prepared.from_id()).await?;
        let expected_public_key = single_active_public_key(&account.active)?;
        prepared.sign_with_wif(wif, &expected_public_key)
    }

    pub async fn wait_for_transfer_confirmation(
        self,
        receipt: &TransferBroadcastReceipt,
    ) -> Result<TransferConfirmation, SwaplockApiError> {
        wait_for_transfer_confirmation(
            self.session,
            receipt,
            DEFAULT_CONFIRMATION_ATTEMPTS,
            DEFAULT_CONFIRMATION_DELAY,
        )
        .await
    }
}

impl TransferConfirmation {
    pub fn history(&self) -> &OperationHistoryObject {
        &self.history
    }

    pub fn id(&self) -> &str {
        &self.history.id.0
    }

    pub fn block_num(&self) -> u32 {
        self.history.block_num
    }

    pub fn trx_in_block(&self) -> u16 {
        self.history.trx_in_block
    }

    pub fn op_in_trx(&self) -> u16 {
        self.history.op_in_trx
    }

    pub fn virtual_op(&self) -> u32 {
        self.history.virtual_op
    }
}

async fn wait_for_transfer_confirmation(
    session: &mut GrapheneSession,
    receipt: &TransferBroadcastReceipt,
    attempts: u32,
    delay: Duration,
) -> Result<TransferConfirmation, SwaplockApiError> {
    for attempt in 0..attempts {
        let page = HistoryApi { session }
            .account_history_by_id(receipt.from_id())
            .limit(DEFAULT_CONFIRMATION_LIMIT)
            .offset(0)
            .get()
            .await?;
        if let Some(history) = page
            .items()
            .iter()
            .find(|history| transfer_history_matches_receipt(history, receipt))
            .cloned()
        {
            return Ok(TransferConfirmation { history });
        }

        if attempt + 1 < attempts {
            thread::sleep(delay);
        }
    }

    Err(SwaplockApiError::TransferConfirmationNotFound {
        from: receipt.from_id().to_string(),
        to: receipt.to_id().to_string(),
        amount: receipt.amount(),
        asset: receipt.asset_id().to_string(),
        min_block_num: receipt.min_block_num(),
    })
}

fn transfer_history_matches_receipt(
    history: &OperationHistoryObject,
    receipt: &TransferBroadcastReceipt,
) -> bool {
    if u64::from(history.block_num) < receipt.min_block_num() {
        return false;
    }
    let Some(operation) = history.op.as_transfer() else {
        return false;
    };

    operation.from.0 == receipt.from_id()
        && operation.to.0 == receipt.to_id()
        && operation.amount.amount == receipt.amount()
        && operation.amount.asset_id.0 == receipt.asset_id()
        && operation.fee.amount == receipt.fee_amount()
        && operation.fee.asset_id.0 == receipt.fee_asset_id()
}

fn single_active_public_key(
    authority: &graphene_chain_swaplock_bindings::generated::Authority,
) -> Result<String, SwaplockApiError> {
    match authority.key_auths.as_slice() {
        [(public_key, weight)] if u32::from(*weight) >= authority.weight_threshold => {
            Ok(public_key.clone())
        }
        [] => Err(SwaplockApiError::InvalidTransfer {
            message: "active authority has no public keys; provide expected public key explicitly"
                .to_string(),
        }),
        [_] => Err(SwaplockApiError::InvalidTransfer {
            message: "single active public key weight is below authority threshold".to_string(),
        }),
        _ => Err(SwaplockApiError::InvalidTransfer {
            message:
                "active authority has multiple public keys; provide expected public key explicitly"
                    .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_chain_swaplock_bindings::generated::ids::{
        AccountId, AssetId, OperationHistoryId,
    };
    use graphene_chain_swaplock_bindings::generated::operations::TransferOperation;
    use graphene_chain_swaplock_bindings::generated::static_variants::{
        Operation, OperationResult,
    };
    use graphene_chain_swaplock_bindings::generated::types::{Asset, Authority, VoidResult};
    use serde_json::json;

    #[test]
    fn accepts_single_sufficient_active_public_key() {
        let authority = Authority {
            weight_threshold: 1,
            account_auths: vec![],
            key_auths: vec![("BTS-public-key".to_string(), 1)],
            address_auths: vec![],
        };

        assert_eq!(
            single_active_public_key(&authority).unwrap(),
            "BTS-public-key"
        );
    }

    #[test]
    fn rejects_missing_active_public_key() {
        let authority = Authority {
            weight_threshold: 1,
            account_auths: vec![],
            key_auths: vec![],
            address_auths: vec![],
        };

        assert_eq!(
            single_active_public_key(&authority)
                .unwrap_err()
                .to_string(),
            "invalid transfer: active authority has no public keys; provide expected public key explicitly"
        );
    }

    #[test]
    fn rejects_multiple_active_public_keys() {
        let authority = Authority {
            weight_threshold: 1,
            account_auths: vec![],
            key_auths: vec![("one".to_string(), 1), ("two".to_string(), 1)],
            address_auths: vec![],
        };

        assert_eq!(
            single_active_public_key(&authority)
                .unwrap_err()
                .to_string(),
            "invalid transfer: active authority has multiple public keys; provide expected public key explicitly"
        );
    }

    #[test]
    fn rejects_insufficient_single_active_public_key_weight() {
        let authority = Authority {
            weight_threshold: 2,
            account_auths: vec![],
            key_auths: vec![("BTS-public-key".to_string(), 1)],
            address_auths: vec![],
        };

        assert_eq!(
            single_active_public_key(&authority)
                .unwrap_err()
                .to_string(),
            "invalid transfer: single active public key weight is below authority threshold"
        );
    }

    #[test]
    fn transfer_history_matches_receipt_fields() {
        let receipt = fixture_receipt(123);
        let history = fixture_history(123, "1.2.100", "1.2.0", 1, "1.3.0", 200_000);

        assert!(transfer_history_matches_receipt(&history, &receipt));
    }

    #[test]
    fn transfer_history_rejects_wrong_amount() {
        let receipt = fixture_receipt(123);
        let history = fixture_history(123, "1.2.100", "1.2.0", 2, "1.3.0", 200_000);

        assert!(!transfer_history_matches_receipt(&history, &receipt));
    }

    #[test]
    fn transfer_history_rejects_block_before_broadcast_head() {
        let receipt = fixture_receipt(123);
        let history = fixture_history(122, "1.2.100", "1.2.0", 1, "1.3.0", 200_000);

        assert!(!transfer_history_matches_receipt(&history, &receipt));
    }

    fn fixture_receipt(min_block_num: u64) -> TransferBroadcastReceipt {
        TransferBroadcastReceipt::from_parts_for_tests(
            json!({"operations": []}),
            "1.2.100".to_string(),
            "1.2.0".to_string(),
            "1.3.0".to_string(),
            1,
            200_000,
            "1.3.0".to_string(),
            min_block_num,
        )
    }

    fn fixture_history(
        block_num: u32,
        from: &str,
        to: &str,
        amount: i64,
        asset_id: &str,
        fee_amount: i64,
    ) -> OperationHistoryObject {
        OperationHistoryObject {
            id: OperationHistoryId("1.11.1".to_string()),
            op: Operation::transfer(TransferOperation {
                fee: Asset::new(fee_amount, AssetId("1.3.0".to_string())),
                from: AccountId(from.to_string()),
                to: AccountId(to.to_string()),
                amount: Asset::new(amount, AssetId(asset_id.to_string())),
                memo: None,
                extensions: vec![],
            }),
            result: OperationResult::VoidResult(Box::new(VoidResult {})),
            block_num,
            trx_in_block: 0,
            op_in_trx: 0,
            virtual_op: 0,
            is_virtual: false,
            block_time: "2026-01-01T00:00:00".to_string(),
        }
    }
}
