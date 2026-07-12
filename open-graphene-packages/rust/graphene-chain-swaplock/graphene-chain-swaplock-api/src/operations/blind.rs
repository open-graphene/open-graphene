//! Thin builders for confidential (blind) transfers: move funds into, between and out of Pedersen
//! commitments.
//!
//! These are deliberately low-level. A blind transfer is only valid when the input commitments plus
//! the fee balance the output commitments, which means the caller owns the cryptography: build the
//! commitments, range proofs and blinding factors with [`CryptoApi`](crate::CryptoApi) (or any
//! libsecp256k1-compatible tool) and hand the finished bytes here. This module shapes them into the
//! operation and prices it; it does not generate or balance the commitments for you.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_swaplock_bindings::generated::operations::{
    BlindTransferOperation, TransferFromBlindOperation, TransferToBlindOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{Asset, BlindInput, BlindOutput};
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::account::single_key_authority;
use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Build a [`BlindOutput`] owned by a single key, with no stealth memo. Bytes are the finished
/// commitment and range proof (from [`CryptoApi`](crate::CryptoApi)).
fn blind_output(
    commitment: impl Into<Vec<u8>>,
    range_proof: impl Into<Vec<u8>>,
    owner_key: String,
) -> BlindOutput {
    BlindOutput {
        commitment: commitment.into(),
        range_proof: range_proof.into(),
        owner: single_key_authority(owner_key),
        stealth_memo: None,
    }
}

/// Build a [`BlindInput`] owned by a single key, spending a finished commitment.
fn blind_input(commitment: impl Into<Vec<u8>>, owner_key: String) -> BlindInput {
    BlindInput {
        commitment: commitment.into(),
        owner: single_key_authority(owner_key),
    }
}

/// Builder for `transfer_to_blind`: move a public `amount` from `from` into blind outputs.
///
/// Required: `from`, the `.amount(..)`, the `.blinding_factor(..)` (the sum of the output blinding
/// factors) and at least one `.output(..)`. The output commitments must balance the public amount.
pub struct TransferToBlindRequest<'session> {
    session: &'session mut GrapheneSession,
    from: String,
    amount: i64,
    asset: String,
    blinding_factor: Option<Vec<u8>>,
    outputs: Vec<BlindOutput>,
}

impl<'session> TransferToBlindRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        from: impl Into<String>,
        amount: i64,
        asset: impl Into<String>,
    ) -> Self {
        Self {
            session,
            from: from.into(),
            amount,
            asset: asset.into(),
            blinding_factor: None,
            outputs: vec![],
        }
    }

    /// The 32-byte blinding factor that balances the outputs against the public amount.
    pub fn blinding_factor(mut self, blinding_factor: impl Into<Vec<u8>>) -> Self {
        self.blinding_factor = Some(blinding_factor.into());
        self
    }

    /// Add a blind output owned by `owner_key`, carrying the given commitment and range proof.
    pub fn output(
        mut self,
        commitment: impl Into<Vec<u8>>,
        range_proof: impl Into<Vec<u8>>,
        owner_key: impl Into<String>,
    ) -> Self {
        self.outputs
            .push(blind_output(commitment, range_proof, owner_key.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let blinding_factor =
            self.blinding_factor
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "blinding_factor",
                })?;
        if self.outputs.is_empty() {
            return Err(SwaplockApiError::MissingTransferField { field: "output" });
        }
        let operation = Operation::transfer_to_blind(TransferToBlindOperation {
            fee: core_fee(),
            amount: Asset::new(self.amount, AssetId(self.asset)),
            from: AccountId(self.from),
            blinding_factor,
            outputs: self.outputs,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `blind_transfer`: move funds between blind commitments (input commitments plus fee
/// balance the outputs).
///
/// Required: at least one `.input(..)` and one `.output(..)`.
pub struct BlindTransferRequest<'session> {
    session: &'session mut GrapheneSession,
    inputs: Vec<BlindInput>,
    outputs: Vec<BlindOutput>,
}

impl<'session> BlindTransferRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession) -> Self {
        Self {
            session,
            inputs: vec![],
            outputs: vec![],
        }
    }

    /// Spend a blind commitment owned by `owner_key`.
    pub fn input(mut self, commitment: impl Into<Vec<u8>>, owner_key: impl Into<String>) -> Self {
        self.inputs.push(blind_input(commitment, owner_key.into()));
        self
    }

    /// Add a blind output owned by `owner_key`, carrying the given commitment and range proof.
    pub fn output(
        mut self,
        commitment: impl Into<Vec<u8>>,
        range_proof: impl Into<Vec<u8>>,
        owner_key: impl Into<String>,
    ) -> Self {
        self.outputs
            .push(blind_output(commitment, range_proof, owner_key.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        if self.inputs.is_empty() {
            return Err(SwaplockApiError::MissingTransferField { field: "input" });
        }
        if self.outputs.is_empty() {
            return Err(SwaplockApiError::MissingTransferField { field: "output" });
        }
        let operation = Operation::blind_transfer(BlindTransferOperation {
            fee: core_fee(),
            inputs: self.inputs,
            outputs: self.outputs,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `transfer_from_blind`: move a public `amount` out of blind inputs to `to`.
///
/// Required: `to`, the `.amount(..)`, the `.blinding_factor(..)` and at least one `.input(..)`. The
/// input commitments must balance the public amount plus the fee.
pub struct TransferFromBlindRequest<'session> {
    session: &'session mut GrapheneSession,
    to: String,
    amount: i64,
    asset: String,
    blinding_factor: Option<Vec<u8>>,
    inputs: Vec<BlindInput>,
}

impl<'session> TransferFromBlindRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        to: impl Into<String>,
        amount: i64,
        asset: impl Into<String>,
    ) -> Self {
        Self {
            session,
            to: to.into(),
            amount,
            asset: asset.into(),
            blinding_factor: None,
            inputs: vec![],
        }
    }

    /// The 32-byte blinding factor that balances the inputs against the public amount and fee.
    pub fn blinding_factor(mut self, blinding_factor: impl Into<Vec<u8>>) -> Self {
        self.blinding_factor = Some(blinding_factor.into());
        self
    }

    /// Spend a blind commitment owned by `owner_key`.
    pub fn input(mut self, commitment: impl Into<Vec<u8>>, owner_key: impl Into<String>) -> Self {
        self.inputs.push(blind_input(commitment, owner_key.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let blinding_factor =
            self.blinding_factor
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "blinding_factor",
                })?;
        if self.inputs.is_empty() {
            return Err(SwaplockApiError::MissingTransferField { field: "input" });
        }
        let operation = Operation::transfer_from_blind(TransferFromBlindOperation {
            fee: core_fee(),
            amount: Asset::new(self.amount, AssetId(self.asset)),
            to: AccountId(self.to),
            blinding_factor,
            inputs: self.inputs,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
