use open_graphene_transport::{GrapheneSession, crypto_api};

use crate::SwaplockApiError;

use super::error::{decode, encode};
use super::types::{BlindingFactor, Commitment, RangeProof};

/// Builder for `crypto.range_proof_sign`: produce a range proof that a committed
/// value lies in a range, without revealing the value.
pub struct RangeProofSignRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) min_value: u64,
    pub(super) commit: Commitment,
    pub(super) commit_blind: BlindingFactor,
    pub(super) nonce: BlindingFactor,
    pub(super) base10_exp: i8,
    pub(super) min_bits: u8,
    pub(super) actual_value: u64,
}

impl RangeProofSignRequest<'_> {
    pub async fn get(self) -> Result<RangeProof, SwaplockApiError> {
        range_proof_sign(
            self.session,
            self.min_value,
            self.commit,
            self.commit_blind,
            self.nonce,
            self.base10_exp,
            self.min_bits,
            self.actual_value,
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn range_proof_sign(
    session: &mut GrapheneSession,
    min_value: u64,
    commit: Commitment,
    commit_blind: BlindingFactor,
    nonce: BlindingFactor,
    base10_exp: i8,
    min_bits: u8,
    actual_value: u64,
) -> Result<RangeProof, SwaplockApiError> {
    let commit = encode("range_proof_sign", &commit)?;
    let commit_blind = encode("range_proof_sign", &commit_blind)?;
    let nonce = encode("range_proof_sign", &nonce)?;
    let result = crypto_api::range_proof_sign(
        session,
        min_value,
        commit,
        commit_blind,
        nonce,
        base10_exp,
        min_bits,
        actual_value,
    )?;
    decode("range_proof_sign", result)
}
