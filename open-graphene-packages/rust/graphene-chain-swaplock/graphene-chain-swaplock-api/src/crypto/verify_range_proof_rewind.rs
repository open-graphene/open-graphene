use open_graphene_transport::{GrapheneSession, crypto_api};

use crate::SwaplockApiError;

use super::error::{decode, encode};
use super::types::{BlindingFactor, Commitment, RangeProof, VerifyRangeProofRewindResult};

/// Builder for `crypto.verify_range_proof_rewind`: verify a proof and, using the
/// nonce, recover the hidden value, blinding factor and message.
pub struct VerifyRangeProofRewindRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) nonce: BlindingFactor,
    pub(super) commit: Commitment,
    pub(super) proof: RangeProof,
}

impl VerifyRangeProofRewindRequest<'_> {
    pub async fn get(self) -> Result<VerifyRangeProofRewindResult, SwaplockApiError> {
        verify_range_proof_rewind(self.session, self.nonce, self.commit, self.proof).await
    }
}

pub(super) async fn verify_range_proof_rewind(
    session: &mut GrapheneSession,
    nonce: BlindingFactor,
    commit: Commitment,
    proof: RangeProof,
) -> Result<VerifyRangeProofRewindResult, SwaplockApiError> {
    let nonce = encode("verify_range_proof_rewind", &nonce)?;
    let commit = encode("verify_range_proof_rewind", &commit)?;
    let proof = encode("verify_range_proof_rewind", &proof)?;
    let result = crypto_api::verify_range_proof_rewind(session, nonce, commit, proof)?;
    decode("verify_range_proof_rewind", result)
}
