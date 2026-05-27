use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::fc::{
    decode_public_key, is_graphene_canonical_compact_signature, verify_compact_signature_public_key,
};
use graphene_chain_swaplock_bindings::generated::ids::{
    AccountId, AssetId, LimitOrderId, PUBLIC_KEY_PREFIX,
};
use graphene_chain_swaplock_bindings::generated::operations::TransferOperation;
use graphene_chain_swaplock_bindings::generated::static_variants::{FutureExtensions, Operation};
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction, Transaction};
use open_graphene_sdk_core::{
    AssetAmount, AssetIdRef, BalanceCheck, HeadBlock, TransactionHeader, decimal_to_raw_amount,
    ensure_sufficient_balance, transaction_header_from_head,
};
use open_graphene_sdk_operations::{
    GrapheneOperationBuilderTypes, SignedTransactionJsonParts, TransferChainTypes, TransferInput,
    build_transfer_transaction_for, signed_transaction_broadcast_json,
};
use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::{DatabaseApi, SwaplockApiError};

const DEFAULT_EXPIRATION: Duration = Duration::from_secs(60);
const DEFAULT_FEE_ASSET_ID: &str = "1.3.0";

pub struct TransferRequest<'session> {
    session: &'session mut GrapheneSession,
    from: Option<String>,
    to: Option<String>,
    amount: Option<TransferAmount>,
    fee_asset: Option<String>,
    max_fee: Option<i64>,
    expiration: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TransferAmount {
    Raw { amount: i64, asset: String },
    Decimal { amount: String, asset: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedTransfer {
    transaction: Transaction,
    from_id: String,
    to_id: String,
    asset_id: String,
    amount: i64,
    fee: Asset,
    balance_before: i64,
    balance_after: i64,
    asset_precision: u8,
    head_block_number: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignedTransfer {
    signed_transaction: SignedTransaction,
    from_id: String,
    to_id: String,
    asset_id: String,
    amount: i64,
    fee: Asset,
    balance_before: i64,
    balance_after: i64,
    asset_precision: u8,
    head_block_number: u64,
}

struct SwaplockOperationBuilderTypes;

impl GrapheneOperationBuilderTypes for SwaplockOperationBuilderTypes {
    type Transaction = Transaction;
    type Operation = Operation;
    type Asset = Asset;
    type AccountId = AccountId;
    type AssetId = AssetId;
    type LimitOrderId = LimitOrderId;
    type FutureExtensions = Vec<FutureExtensions>;

    fn asset(amount: i64, asset_id: Self::AssetId) -> Self::Asset {
        Asset::new(amount, asset_id)
    }

    fn empty_extensions() -> Self::FutureExtensions {
        vec![]
    }

    fn transaction(
        header: TransactionHeader,
        operations: Vec<Self::Operation>,
        extensions: Self::FutureExtensions,
    ) -> Self::Transaction {
        Transaction {
            ref_block_num: header.ref_block_num,
            ref_block_prefix: header.ref_block_prefix,
            expiration: header.expiration,
            operations,
            extensions,
        }
    }
}

impl TransferChainTypes for SwaplockOperationBuilderTypes {
    type TransferOperation = TransferOperation;

    fn transfer_operation_without_memo(
        fee: Self::Asset,
        from: Self::AccountId,
        to: Self::AccountId,
        amount: Self::Asset,
        extensions: Self::FutureExtensions,
    ) -> Self::TransferOperation {
        TransferOperation {
            fee,
            from,
            to,
            amount,
            memo: None,
            extensions,
        }
    }

    fn operation_transfer(operation: Self::TransferOperation) -> Self::Operation {
        Operation::transfer(operation)
    }
}

impl<'session> TransferRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession) -> Self {
        Self {
            session,
            from: None,
            to: None,
            amount: None,
            fee_asset: None,
            max_fee: None,
            expiration: DEFAULT_EXPIRATION,
        }
    }

    pub fn from(mut self, account_name_or_id: impl Into<String>) -> Self {
        self.from = Some(account_name_or_id.into());
        self
    }

    pub fn to(mut self, account_name_or_id: impl Into<String>) -> Self {
        self.to = Some(account_name_or_id.into());
        self
    }

    pub fn amount_raw(mut self, amount: i64, asset_symbol_or_id: impl Into<String>) -> Self {
        self.amount = Some(TransferAmount::Raw {
            amount,
            asset: asset_symbol_or_id.into(),
        });
        self
    }

    pub fn amount_decimal(
        mut self,
        amount: impl Into<String>,
        asset_symbol_or_id: impl Into<String>,
    ) -> Self {
        self.amount = Some(TransferAmount::Decimal {
            amount: amount.into(),
            asset: asset_symbol_or_id.into(),
        });
        self
    }

    pub fn fee_asset(mut self, asset_symbol_or_id: impl Into<String>) -> Self {
        self.fee_asset = Some(asset_symbol_or_id.into());
        self
    }

    pub fn max_fee_raw(mut self, amount: i64) -> Self {
        self.max_fee = Some(amount);
        self
    }

    pub fn expiration(mut self, expiration: Duration) -> Self {
        self.expiration = expiration;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransfer, SwaplockApiError> {
        let from = self
            .from
            .ok_or(SwaplockApiError::MissingTransferField { field: "from" })?;
        let to = self
            .to
            .ok_or(SwaplockApiError::MissingTransferField { field: "to" })?;
        let amount = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let max_fee = self
            .max_fee
            .ok_or(SwaplockApiError::MissingTransferField { field: "max_fee" })?;

        let mut database = DatabaseApi {
            session: self.session,
        };
        let from_account = resolve_account(&mut database, &from).await?;
        let to_account = resolve_account(&mut database, &to).await?;
        let asset_ref = amount.asset().to_string();
        let asset = resolve_asset(&mut database, &asset_ref).await?;
        let fee_asset_ref = self
            .fee_asset
            .as_deref()
            .unwrap_or(DEFAULT_FEE_ASSET_ID)
            .to_string();
        let fee_asset = resolve_asset(&mut database, &fee_asset_ref).await?;
        let amount_raw = amount.raw_amount(asset.precision)?;
        let properties = database.get_dynamic_global_properties().await?;
        let header = transaction_header_from_head(
            &HeadBlock {
                number: properties.head_block_number as u64,
                id: hex(&properties.head_block_id),
                time: properties.time.clone(),
            },
            self.expiration,
        )?;
        let head_block_number = properties.head_block_number as u64;

        let mut transaction =
            build_transfer_transaction_for::<SwaplockOperationBuilderTypes>(TransferInput::new(
                header,
                from_account.id.0.clone(),
                to_account.id.0.clone(),
                amount_raw,
                asset.id.0.clone(),
                0,
                fee_asset.id.0.clone(),
            ));
        let required_fee = required_transfer_fee(database.session, &transaction, &fee_asset.id.0)?;
        if required_fee.amount > max_fee {
            return Err(SwaplockApiError::TransferFeeTooHigh {
                required: required_fee.amount,
                max: max_fee,
            });
        }
        set_transfer_fee(&mut transaction, required_fee.clone())?;

        let balances = database
            .get_account_balances_by_id(&from_account.id.0, vec![asset.id.0.clone()])
            .await?;
        let balance_before = balances.first().map(|balance| balance.amount).unwrap_or(0);
        let balance_after = if asset.id.0 == required_fee.asset_id.0 {
            balance_before - amount_raw - required_fee.amount
        } else {
            balance_before - amount_raw
        };
        ensure_sufficient_balance(&BalanceCheck {
            balance: AssetAmount::new(balance_before, AssetIdRef::parse(&asset.id.0)?),
            transfer_amount: AssetAmount::new(amount_raw, AssetIdRef::parse(&asset.id.0)?),
            fee: AssetAmount::new(
                required_fee.amount,
                AssetIdRef::parse(&required_fee.asset_id.0)?,
            ),
        })?;

        Ok(PreparedTransfer {
            transaction,
            from_id: from_account.id.0,
            to_id: to_account.id.0,
            asset_id: asset.id.0,
            amount: amount_raw,
            fee: required_fee,
            balance_before,
            balance_after,
            asset_precision: asset.precision,
            head_block_number,
        })
    }
}

impl TransferAmount {
    fn asset(&self) -> &str {
        match self {
            Self::Raw { asset, .. } | Self::Decimal { asset, .. } => asset,
        }
    }

    fn raw_amount(&self, precision: u8) -> Result<i64, SwaplockApiError> {
        match self {
            Self::Raw { amount, .. } => Ok(*amount),
            Self::Decimal { amount, .. } => Ok(decimal_to_raw_amount(amount, precision)?),
        }
    }
}

impl PreparedTransfer {
    pub fn transaction(&self) -> &Transaction {
        &self.transaction
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

    pub fn fee(&self) -> &Asset {
        &self.fee
    }

    pub fn balance_before(&self) -> i64 {
        self.balance_before
    }

    pub fn balance_after(&self) -> i64 {
        self.balance_after
    }

    pub fn asset_precision(&self) -> u8 {
        self.asset_precision
    }

    pub fn head_block_number(&self) -> u64 {
        self.head_block_number
    }

    pub fn sign_with_wif(
        self,
        wif: &str,
        expected_public_key: &str,
    ) -> Result<SignedTransfer, SwaplockApiError> {
        let signed_transaction =
            sign_transfer_transaction_checked(&self.transaction, wif, expected_public_key)?;

        Ok(SignedTransfer {
            signed_transaction,
            from_id: self.from_id,
            to_id: self.to_id,
            asset_id: self.asset_id,
            amount: self.amount,
            fee: self.fee,
            balance_before: self.balance_before,
            balance_after: self.balance_after,
            asset_precision: self.asset_precision,
            head_block_number: self.head_block_number,
        })
    }
}

impl SignedTransfer {
    pub fn signed_transaction(&self) -> &SignedTransaction {
        &self.signed_transaction
    }

    pub fn transaction_json(&self) -> Result<Value, SwaplockApiError> {
        signed_transfer_json(&self.signed_transaction)
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

    pub fn fee(&self) -> &Asset {
        &self.fee
    }

    pub fn balance_before(&self) -> i64 {
        self.balance_before
    }

    pub fn balance_after(&self) -> i64 {
        self.balance_after
    }

    pub fn asset_precision(&self) -> u8 {
        self.asset_precision
    }

    pub fn head_block_number(&self) -> u64 {
        self.head_block_number
    }
}

async fn resolve_account(
    database: &mut DatabaseApi<'_>,
    account_name_or_id: &str,
) -> Result<graphene_chain_swaplock_bindings::generated::AccountObject, SwaplockApiError> {
    if account_name_or_id.starts_with("1.2.") {
        database.get_account_by_id(account_name_or_id).await
    } else {
        database.get_account_by_name(account_name_or_id).await
    }
}

async fn resolve_asset(
    database: &mut DatabaseApi<'_>,
    asset_symbol_or_id: &str,
) -> Result<graphene_chain_swaplock_bindings::generated::AssetObject, SwaplockApiError> {
    if asset_symbol_or_id.starts_with("1.3.") {
        database.get_asset_by_id(asset_symbol_or_id).await
    } else {
        database.get_asset_by_symbol(asset_symbol_or_id).await
    }
}

fn required_transfer_fee(
    session: &mut GrapheneSession,
    transaction: &Transaction,
    fee_asset_id: &str,
) -> Result<Asset, SwaplockApiError> {
    let operation_json = transaction
        .operations
        .first()
        .ok_or(SwaplockApiError::InvalidTransfer {
            message: "transaction contains no operations".to_string(),
        })
        .and_then(transfer_operation_json)?;
    let value =
        session.database_call("get_required_fees", json!([[operation_json], fee_asset_id]))?;
    let fees = value
        .as_array()
        .ok_or_else(|| SwaplockApiError::UnexpectedResponse {
            method: "get_required_fees",
            message: "expected fee array".to_string(),
        })?;
    let fee = fees
        .first()
        .cloned()
        .ok_or_else(|| SwaplockApiError::UnexpectedResponse {
            method: "get_required_fees",
            message: "missing transfer fee".to_string(),
        })?;

    serde_json::from_value(fee).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: "get_required_fees",
        message: error.to_string(),
    })
}

fn set_transfer_fee(transaction: &mut Transaction, fee: Asset) -> Result<(), SwaplockApiError> {
    let operation =
        transaction
            .operations
            .first_mut()
            .ok_or(SwaplockApiError::InvalidTransfer {
                message: "transaction contains no operations".to_string(),
            })?;
    let Operation::TransferOperation(operation) = operation else {
        return Err(SwaplockApiError::InvalidTransfer {
            message: "first operation is not a transfer".to_string(),
        });
    };
    operation.fee = fee;
    Ok(())
}

fn sign_transfer_transaction_checked(
    transaction: &Transaction,
    wif: &str,
    expected_public_key: &str,
) -> Result<SignedTransaction, SwaplockApiError> {
    let signed_transaction =
        transaction
            .signed_with_wif(wif)
            .map_err(|_| SwaplockApiError::InvalidTransfer {
                message: "failed to sign transfer transaction".to_string(),
            })?;
    let signature =
        signed_transaction
            .signatures
            .first()
            .ok_or_else(|| SwaplockApiError::InvalidTransfer {
                message: "signed transfer transaction has no signature".to_string(),
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
    let matches_public_key = verify_compact_signature_public_key(digest, &signature.0, public_key)
        .map_err(|error| SwaplockApiError::InvalidTransfer {
            message: format!("failed to verify signature public key: {error}"),
        })?;
    if !matches_public_key {
        return Err(SwaplockApiError::InvalidTransfer {
            message: "signature public key verification failed".to_string(),
        });
    }

    Ok(signed_transaction)
}

fn signed_transfer_json(signed_transaction: &SignedTransaction) -> Result<Value, SwaplockApiError> {
    let operations = signed_transaction
        .operations
        .iter()
        .map(transfer_operation_json)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(signed_transaction_broadcast_json(
        SignedTransactionJsonParts {
            ref_block_num: signed_transaction.ref_block_num,
            ref_block_prefix: signed_transaction.ref_block_prefix,
            expiration: &signed_transaction.expiration,
            operations,
            signatures: signed_transaction
                .signatures
                .iter()
                .map(|signature| signature.0.as_slice())
                .collect(),
        },
    ))
}

fn transfer_operation_json(operation: &Operation) -> Result<Value, SwaplockApiError> {
    let operation = operation
        .as_transfer()
        .ok_or(SwaplockApiError::InvalidTransfer {
            message: "operation is not a transfer".to_string(),
        })?;

    Ok(json!([
        0,
        {
            "fee": asset_json(&operation.fee),
            "from": operation.from.0,
            "to": operation.to.0,
            "amount": asset_json(&operation.amount),
            "memo": null,
            "extensions": []
        }
    ]))
}

fn asset_json(asset: &Asset) -> Value {
    json!({
        "amount": asset.amount,
        "asset_id": asset.asset_id.0,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_chain_swaplock_bindings::generated::ids::AssetId;

    const FIXTURE_WIF: &str = "5J4eFhjREJA7hKG6KcvHofHMXyGQZCDpQE463PAaKo9xXY6UDPq";
    const FIXTURE_PUBLIC_KEY: &str = "BTS7jDPoMwyjVH5obFmqzFNp4Ffp7G2nvC7FKFkrMBpo7Sy4uq5Mj";
    const WRONG_PUBLIC_KEY: &str = "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV";

    #[test]
    fn transfer_operation_json_matches_graphene_wire_shape() {
        let transaction =
            build_transfer_transaction_for::<SwaplockOperationBuilderTypes>(TransferInput::new(
                TransactionHeader {
                    ref_block_num: 2,
                    ref_block_prefix: 3,
                    expiration: "2026-05-25T12:01:00".to_string(),
                },
                "1.2.100",
                "1.2.101",
                100_000,
                "1.3.0",
                0,
                "1.3.0",
            ));

        assert_eq!(
            transfer_operation_json(transaction.operations.first().unwrap()).unwrap(),
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
    fn set_transfer_fee_updates_transfer_operation() {
        let mut transaction = fixture_transfer_transaction();

        set_transfer_fee(
            &mut transaction,
            Asset::new(42, AssetId("1.3.0".to_string())),
        )
        .unwrap();

        let Operation::TransferOperation(operation) = transaction.operations.first().unwrap()
        else {
            panic!("fixture should contain transfer");
        };
        assert_eq!(operation.fee.amount, 42);
    }

    #[test]
    fn signs_prepared_transfer_when_expected_public_key_matches() {
        let prepared = fixture_prepared_transfer();

        let signed = prepared
            .sign_with_wif(FIXTURE_WIF, FIXTURE_PUBLIC_KEY)
            .expect("fixture key signs transfer");

        assert_eq!(signed.signed_transaction().signatures.len(), 1);
        assert_eq!(signed.from_id(), "1.2.100");
        assert_eq!(signed.to_id(), "1.2.101");
        assert_eq!(signed.amount(), 100_000);
    }

    #[test]
    fn rejects_signature_when_expected_public_key_does_not_match() {
        let err = fixture_prepared_transfer()
            .sign_with_wif(FIXTURE_WIF, WRONG_PUBLIC_KEY)
            .expect_err("wrong public key fails verification");

        assert_eq!(
            err.to_string(),
            "invalid transfer: signature public key verification failed"
        );
    }

    #[test]
    fn invalid_wif_error_does_not_echo_secret_like_input() {
        let secret_like_value = "not-a-wif-secret-like-value";
        let err = fixture_prepared_transfer()
            .sign_with_wif(secret_like_value, FIXTURE_PUBLIC_KEY)
            .expect_err("invalid WIF fails");

        assert!(!err.to_string().contains(secret_like_value));
        assert_eq!(
            err.to_string(),
            "invalid transfer: failed to sign transfer transaction"
        );
    }

    #[test]
    fn signed_transfer_json_matches_broadcast_shape() {
        let signed = fixture_prepared_transfer()
            .sign_with_wif(FIXTURE_WIF, FIXTURE_PUBLIC_KEY)
            .expect("fixture key signs transfer");
        let value = signed.transaction_json().unwrap();

        assert_eq!(value["ref_block_num"], json!(2));
        assert_eq!(value["ref_block_prefix"], json!(3));
        assert_eq!(value["expiration"], json!("2026-05-25T12:01:00"));
        assert_eq!(value["operations"][0][0], json!(0));
        assert_eq!(value["operations"][0][1]["from"], json!("1.2.100"));
        assert_eq!(value["operations"][0][1]["to"], json!("1.2.101"));
        assert_eq!(
            value["operations"][0][1]["amount"]["amount"],
            json!(100_000)
        );
        assert_eq!(value["operations"][0][1]["fee"]["amount"], json!(200_000));
        assert_eq!(value["signatures"].as_array().unwrap().len(), 1);
    }

    fn fixture_prepared_transfer() -> PreparedTransfer {
        let mut transaction = fixture_transfer_transaction();
        let fee = Asset::new(200_000, AssetId("1.3.0".to_string()));
        set_transfer_fee(&mut transaction, fee.clone()).unwrap();

        PreparedTransfer {
            transaction,
            from_id: "1.2.100".to_string(),
            to_id: "1.2.101".to_string(),
            asset_id: "1.3.0".to_string(),
            amount: 100_000,
            fee,
            balance_before: 1_000_000,
            balance_after: 700_000,
            asset_precision: 5,
            head_block_number: 123,
        }
    }

    fn fixture_transfer_transaction() -> Transaction {
        build_transfer_transaction_for::<SwaplockOperationBuilderTypes>(TransferInput::new(
            TransactionHeader {
                ref_block_num: 2,
                ref_block_prefix: 3,
                expiration: "2026-05-25T12:01:00".to_string(),
            },
            "1.2.100",
            "1.2.101",
            100_000,
            "1.3.0",
            0,
            "1.3.0",
        ))
    }
}
