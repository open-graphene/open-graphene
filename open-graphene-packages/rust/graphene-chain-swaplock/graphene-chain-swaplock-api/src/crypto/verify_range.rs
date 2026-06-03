use open_graphene_transport::{GrapheneSession, crypto_api};

use crate::SwaplockApiError;

use super::error::{decode, encode};
use super::types::{Commitment, RangeProof, VerifyRangeResult};

/// Builder for `crypto.verify_range`: check a range proof for a commitment and
/// learn the `[min_val, max_val]` bounds it proves.
pub struct VerifyRangeRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) commit: Commitment,
    pub(super) proof: RangeProof,
}

impl VerifyRangeRequest<'_> {
    pub async fn get(self) -> Result<VerifyRangeResult, SwaplockApiError> {
        verify_range(self.session, self.commit, self.proof).await
    }
}

pub(super) async fn verify_range(
    session: &mut GrapheneSession,
    commit: Commitment,
    proof: RangeProof,
) -> Result<VerifyRangeResult, SwaplockApiError> {
    let commit = encode("verify_range", &commit)?;
    let proof = encode("verify_range", &proof)?;
    let result = crypto_api::verify_range(session, commit, proof)?;
    decode("verify_range", result)
}
