//! Ergonomic builders for hashed time-locked contracts: lock funds under a hash, release with the
//! preimage.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). `htlc_create`
//! locks an amount that only someone who knows the preimage can claim before the deadline; after
//! the deadline the sender can refund. `htlc_redeem` claims it by revealing the preimage.

use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, HtlcId};
use graphene_chain_swaplock_bindings::generated::operations::{
    HtlcCreateOperation, HtlcExtendOperation, HtlcRedeemOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::{HtlcHash, Operation};
use graphene_chain_swaplock_bindings::generated::types::{
    Asset, HtlcCreateOperationAdditionalOptionsType,
};
use open_graphene_transport::GrapheneSession;
use sha2::{Digest, Sha256};

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

/// How long the contract stays claimable if not set; after it, the sender can refund.
const DEFAULT_CLAIM_PERIOD: Duration = Duration::from_secs(3600);

/// Builder for `htlc_create`: lock `amount` from `from` to `to`, claimable with the preimage.
pub struct HtlcCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    from: String,
    to: String,
    amount: Option<(i64, String)>,
    preimage: Option<Vec<u8>>,
    claim_period: Duration,
}

impl<'session> HtlcCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        Self {
            session,
            from: from.into(),
            to: to.into(),
            amount: None,
            preimage: None,
            claim_period: DEFAULT_CLAIM_PERIOD,
        }
    }

    /// What to lock: raw `amount` of asset id `asset_id`.
    pub fn amount(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.amount = Some((amount, asset_id.into()));
        self
    }

    /// Lock under the SHA-256 of this secret. Keep the secret to redeem with later.
    pub fn lock_sha256(mut self, preimage: impl AsRef<[u8]>) -> Self {
        self.preimage = Some(preimage.as_ref().to_vec());
        self
    }

    /// How long the funds stay claimable before the sender may refund (default one hour).
    pub fn claim_period(mut self, claim_period: Duration) -> Self {
        self.claim_period = claim_period;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset_id) = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let preimage = self
            .preimage
            .ok_or(SwaplockApiError::MissingTransferField {
                field: "lock_sha256",
            })?;

        let digest = Sha256::digest(&preimage).to_vec();
        let operation = Operation::htlc_create(HtlcCreateOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            from: AccountId(self.from),
            to: AccountId(self.to),
            amount: Asset::new(amount, AssetId(asset_id)),
            preimage_hash: HtlcHash::HtlcAlgoSha256(Box::new(digest)),
            preimage_size: preimage.len() as u16,
            claim_period_seconds: self.claim_period.as_secs() as u32,
            extensions: HtlcCreateOperationAdditionalOptionsType { memo: None },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `htlc_redeem`: claim a contract by revealing its preimage.
pub struct HtlcRedeemRequest<'session> {
    session: &'session mut GrapheneSession,
    htlc: String,
    redeemer: String,
    preimage: Option<Vec<u8>>,
}

impl<'session> HtlcRedeemRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        htlc: impl Into<String>,
        redeemer: impl Into<String>,
    ) -> Self {
        Self {
            session,
            htlc: htlc.into(),
            redeemer: redeemer.into(),
            preimage: None,
        }
    }

    /// The secret whose hash locked the contract.
    pub fn preimage(mut self, preimage: impl AsRef<[u8]>) -> Self {
        self.preimage = Some(preimage.as_ref().to_vec());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let preimage = self
            .preimage
            .ok_or(SwaplockApiError::MissingTransferField { field: "preimage" })?;

        let operation = Operation::htlc_redeem(HtlcRedeemOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            htlc_id: HtlcId(self.htlc),
            redeemer: AccountId(self.redeemer),
            preimage,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `htlc_extend`: push an existing contract's deadline further out.
pub struct HtlcExtendRequest<'session> {
    session: &'session mut GrapheneSession,
    htlc: String,
    update_issuer: String,
    seconds_to_add: u32,
}

impl<'session> HtlcExtendRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        htlc: impl Into<String>,
        update_issuer: impl Into<String>,
    ) -> Self {
        Self {
            session,
            htlc: htlc.into(),
            update_issuer: update_issuer.into(),
            seconds_to_add: 0,
        }
    }

    /// How much longer the contract stays claimable, added to its current deadline.
    pub fn extend_by(mut self, duration: Duration) -> Self {
        self.seconds_to_add = duration.as_secs() as u32;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::htlc_extend(HtlcExtendOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            htlc_id: HtlcId(self.htlc),
            update_issuer: AccountId(self.update_issuer),
            seconds_to_add: self.seconds_to_add,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
