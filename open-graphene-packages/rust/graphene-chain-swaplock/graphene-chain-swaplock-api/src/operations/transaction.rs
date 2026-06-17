//! Generic transaction builder: put any supported operation on the wire.
//!
//! Build one or more operations, let the node price them, sign, and hand the JSON to
//! `network_broadcast`. The same flow that `transfer` uses, but for any operation the bindings can
//! serialize. Adding a new operation means one match arm in [`set_operation_fee`].

use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::fc::{
    decode_public_key, is_graphene_canonical_compact_signature, verify_compact_signature_public_key,
};
use graphene_chain_swaplock_bindings::generated::ids::PUBLIC_KEY_PREFIX;
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction, Transaction};
use open_graphene_core::{HeadBlock, transaction_header_from_head};
use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::{DatabaseApi, SwaplockApiError};

const DEFAULT_EXPIRATION: Duration = Duration::from_secs(60);
const DEFAULT_FEE_ASSET_ID: &str = "1.3.0";

/// Builder for a signed transaction carrying one or more operations.
pub struct TransactionBuilder<'session> {
    session: &'session mut GrapheneSession,
    operations: Vec<Operation>,
    fee_asset: Option<String>,
    expiration: Duration,
}

impl<'session> TransactionBuilder<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession) -> Self {
        Self {
            session,
            operations: Vec::new(),
            fee_asset: None,
            expiration: DEFAULT_EXPIRATION,
        }
    }

    /// Add an operation to the transaction. Construct it from the binding types, e.g.
    /// `Operation::limit_order_cancel(...)`. Leave its `fee` at zero; the node sets it.
    pub fn add_operation(mut self, operation: Operation) -> Self {
        self.operations.push(operation);
        self
    }

    /// Pay fees in this asset (defaults to the core asset `1.3.0`).
    pub fn fee_asset(mut self, asset_id: impl Into<String>) -> Self {
        self.fee_asset = Some(asset_id.into());
        self
    }

    /// How long the transaction stays valid after the head block (default 60s).
    pub fn expiration(mut self, expiration: Duration) -> Self {
        self.expiration = expiration;
        self
    }

    /// Fetch the head block, ask the node for the fees, write them into the operations, and
    /// return a transaction ready to sign.
    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        if self.operations.is_empty() {
            return Err(SwaplockApiError::InvalidTransfer {
                message: "transaction has no operations".to_string(),
            });
        }
        let fee_asset_id = self
            .fee_asset
            .unwrap_or_else(|| DEFAULT_FEE_ASSET_ID.to_string());

        let mut database = DatabaseApi {
            session: self.session,
        };
        let properties = database.get_dynamic_global_properties().await?;
        let header = transaction_header_from_head(
            &HeadBlock {
                number: properties.head_block_number as u64,
                id: hex::encode(&properties.head_block_id),
                time: properties.time.clone(),
            },
            self.expiration,
        )?;

        let mut operations = self.operations;
        let fees = required_fees(database.session, &operations, &fee_asset_id)?;
        for (operation, fee) in operations.iter_mut().zip(fees) {
            set_operation_fee(operation, fee)?;
        }

        Ok(PreparedTransaction {
            transaction: Transaction {
                ref_block_num: header.ref_block_num,
                ref_block_prefix: header.ref_block_prefix,
                expiration: header.expiration,
                operations,
                extensions: vec![],
            },
        })
    }
}

/// A priced transaction waiting for a signature.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedTransaction {
    transaction: Transaction,
}

impl PreparedTransaction {
    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    /// Sign with a WIF private key, checking the signature really belongs to `expected_public_key`.
    pub fn sign_with_wif(
        self,
        wif: &str,
        expected_public_key: &str,
    ) -> Result<SignedTransactionEnvelope, SwaplockApiError> {
        let signed = sign_checked(&self.transaction, wif, expected_public_key)?;
        Ok(SignedTransactionEnvelope { signed })
    }
}

/// A signed transaction, ready for `network_broadcast`.
#[derive(Clone, Debug, PartialEq)]
pub struct SignedTransactionEnvelope {
    signed: SignedTransaction,
}

impl SignedTransactionEnvelope {
    pub fn signed_transaction(&self) -> &SignedTransaction {
        &self.signed
    }

    /// The JSON to pass to `network_broadcast().broadcast_transaction(..)`.
    pub fn transaction_json(&self) -> Result<Value, SwaplockApiError> {
        let operations = operations_json(&self.signed.operations)?;
        Ok(json!({
            "ref_block_num": self.signed.ref_block_num,
            "ref_block_prefix": self.signed.ref_block_prefix,
            "expiration": self.signed.expiration,
            "operations": operations,
            "extensions": [],
            "signatures": self.signed
                .signatures
                .iter()
                .map(|signature| hex::encode(&signature.0))
                .collect::<Vec<_>>(),
        }))
    }
}

/// Write the node-supplied `fee` into an operation. Add a match arm here to support a new operation.
pub(super) fn set_operation_fee(
    operation: &mut Operation,
    fee: Asset,
) -> Result<(), SwaplockApiError> {
    match operation {
        Operation::TransferOperation(operation) => operation.fee = fee,
        Operation::LimitOrderCreateOperation(operation) => operation.fee = fee,
        Operation::LimitOrderCancelOperation(operation) => operation.fee = fee,
        Operation::LimitOrderUpdateOperation(operation) => operation.fee = fee,
        Operation::AssetIssueOperation(operation) => operation.fee = fee,
        Operation::AssetReserveOperation(operation) => operation.fee = fee,
        Operation::AssetUpdateOperation(operation) => operation.fee = fee,
        Operation::AccountUpdateOperation(operation) => operation.fee = fee,
        Operation::AccountCreateOperation(operation) => operation.fee = fee,
        Operation::AccountUpgradeOperation(operation) => operation.fee = fee,
        Operation::AccountWhitelistOperation(operation) => operation.fee = fee,
        Operation::AccountTransferOperation(operation) => operation.fee = fee,
        Operation::HtlcCreateOperation(operation) => operation.fee = fee,
        Operation::HtlcRedeemOperation(operation) => operation.fee = fee,
        Operation::HtlcExtendOperation(operation) => operation.fee = fee,
        Operation::ProposalCreateOperation(operation) => operation.fee = fee,
        Operation::LiquidityPoolCreateOperation(operation) => operation.fee = fee,
        Operation::LiquidityPoolDeleteOperation(operation) => operation.fee = fee,
        Operation::LiquidityPoolDepositOperation(operation) => operation.fee = fee,
        Operation::LiquidityPoolWithdrawOperation(operation) => operation.fee = fee,
        Operation::LiquidityPoolExchangeOperation(operation) => operation.fee = fee,
        Operation::LiquidityPoolUpdateOperation(operation) => operation.fee = fee,
        Operation::CallOrderUpdateOperation(operation) => operation.fee = fee,
        other => {
            return Err(SwaplockApiError::InvalidTransfer {
                message: format!(
                    "automatic fee is not wired for this operation yet: {}",
                    operation_name(other)
                ),
            });
        }
    }
    Ok(())
}

fn operation_name(operation: &Operation) -> &'static str {
    match operation {
        Operation::TransferOperation(_) => "transfer",
        Operation::LimitOrderCreateOperation(_) => "limit_order_create",
        Operation::LimitOrderCancelOperation(_) => "limit_order_cancel",
        Operation::LimitOrderUpdateOperation(_) => "limit_order_update",
        Operation::AssetIssueOperation(_) => "asset_issue",
        Operation::AssetReserveOperation(_) => "asset_reserve",
        Operation::AssetUpdateOperation(_) => "asset_update",
        Operation::AccountUpdateOperation(_) => "account_update",
        Operation::AccountCreateOperation(_) => "account_create",
        Operation::AccountUpgradeOperation(_) => "account_upgrade",
        Operation::AccountWhitelistOperation(_) => "account_whitelist",
        Operation::AccountTransferOperation(_) => "account_transfer",
        Operation::HtlcCreateOperation(_) => "htlc_create",
        Operation::HtlcRedeemOperation(_) => "htlc_redeem",
        Operation::HtlcExtendOperation(_) => "htlc_extend",
        Operation::ProposalCreateOperation(_) => "proposal_create",
        Operation::LiquidityPoolCreateOperation(_) => "liquidity_pool_create",
        Operation::LiquidityPoolDeleteOperation(_) => "liquidity_pool_delete",
        Operation::LiquidityPoolDepositOperation(_) => "liquidity_pool_deposit",
        Operation::LiquidityPoolWithdrawOperation(_) => "liquidity_pool_withdraw",
        Operation::LiquidityPoolExchangeOperation(_) => "liquidity_pool_exchange",
        Operation::LiquidityPoolUpdateOperation(_) => "liquidity_pool_update",
        Operation::CallOrderUpdateOperation(_) => "call_order_update",
        _ => "unknown",
    }
}

/// Serialize each operation to its `[op_id, body]` wire form via serde.
fn operations_json(operations: &[Operation]) -> Result<Vec<Value>, SwaplockApiError> {
    operations
        .iter()
        .map(|operation| {
            serde_json::to_value(operation).map_err(|error| SwaplockApiError::UnexpectedResponse {
                method: "operation",
                message: error.to_string(),
            })
        })
        .collect()
}

pub(super) fn required_fees(
    session: &mut GrapheneSession,
    operations: &[Operation],
    fee_asset_id: &str,
) -> Result<Vec<Asset>, SwaplockApiError> {
    let operations_json = operations_json(operations)?;
    let value =
        session.database_call("get_required_fees", json!([operations_json, fee_asset_id]))?;
    // The node returns one fee per operation, but for ops with sub-operations (proposal_create)
    // it returns a `[base_fee, [sub_fees...]]` pair instead of a bare asset. The op itself pays the
    // base fee, so take the first element when an entry is the pair form.
    let raw: Vec<Value> =
        serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
            method: "get_required_fees",
            message: error.to_string(),
        })?;
    raw.into_iter()
        .map(|entry| {
            let fee = match entry {
                Value::Array(items) => items.into_iter().next().ok_or_else(|| {
                    SwaplockApiError::UnexpectedResponse {
                        method: "get_required_fees",
                        message: "empty fee pair".to_string(),
                    }
                })?,
                other => other,
            };
            serde_json::from_value(fee).map_err(|error| SwaplockApiError::UnexpectedResponse {
                method: "get_required_fees",
                message: error.to_string(),
            })
        })
        .collect()
}

fn sign_checked(
    transaction: &Transaction,
    wif: &str,
    expected_public_key: &str,
) -> Result<SignedTransaction, SwaplockApiError> {
    let signed =
        transaction
            .signed_with_wif(wif)
            .map_err(|_| SwaplockApiError::InvalidTransfer {
                message: "failed to sign transaction".to_string(),
            })?;
    let signature = signed
        .signatures
        .first()
        .ok_or_else(|| SwaplockApiError::InvalidTransfer {
            message: "signed transaction has no signature".to_string(),
        })?;
    if !is_graphene_canonical_compact_signature(&signature.0) {
        return Err(SwaplockApiError::InvalidTransfer {
            message: "signature is not Graphene canonical".to_string(),
        });
    }

    let digest = transaction.signature_digest_bytes().map_err(|error| {
        SwaplockApiError::InvalidTransfer {
            message: format!("failed to compute signature digest: {error}"),
        }
    })?;
    let public_key =
        decode_public_key(expected_public_key, Some(PUBLIC_KEY_PREFIX)).map_err(|_| {
            SwaplockApiError::InvalidTransfer {
                message: "invalid expected public key".to_string(),
            }
        })?;
    if !verify_compact_signature_public_key(digest, &signature.0, public_key).map_err(|error| {
        SwaplockApiError::InvalidTransfer {
            message: format!("failed to verify signature public key: {error}"),
        }
    })? {
        return Err(SwaplockApiError::InvalidTransfer {
            message: "signature public key verification failed".to_string(),
        });
    }
    Ok(signed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, LimitOrderId};
    use graphene_chain_swaplock_bindings::generated::operations::{
        LimitOrderCancelOperation, TransferOperation,
    };

    #[test]
    fn transfer_serializes_to_the_graphene_wire_shape() {
        let operation = Operation::transfer(TransferOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            from: AccountId("1.2.100".to_string()),
            to: AccountId("1.2.101".to_string()),
            amount: Asset::new(100_000, AssetId("1.3.0".to_string())),
            memo: None,
            extensions: vec![],
        });

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([0, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "from": "1.2.100",
                "to": "1.2.101",
                "amount": {"amount": 100000, "asset_id": "1.3.0"},
                "memo": null,
                "extensions": []
            }])
        );
    }

    #[test]
    fn limit_order_cancel_serializes_with_tag_two() {
        let operation = Operation::limit_order_cancel(LimitOrderCancelOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            fee_paying_account: AccountId("1.2.100".to_string()),
            order: LimitOrderId("1.7.42".to_string()),
            extensions: vec![],
        });

        assert_eq!(
            serde_json::to_value(&operation).unwrap(),
            json!([2, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "fee_paying_account": "1.2.100",
                "order": "1.7.42",
                "extensions": []
            }])
        );
    }

    #[test]
    fn set_operation_fee_writes_into_a_cancel() {
        let mut operation = Operation::limit_order_cancel(LimitOrderCancelOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            fee_paying_account: AccountId("1.2.100".to_string()),
            order: LimitOrderId("1.7.42".to_string()),
            extensions: vec![],
        });

        set_operation_fee(&mut operation, Asset::new(55, AssetId("1.3.0".to_string()))).unwrap();

        let Operation::LimitOrderCancelOperation(cancel) = &operation else {
            panic!("expected cancel");
        };
        assert_eq!(cancel.fee.amount, 55);
    }
}
