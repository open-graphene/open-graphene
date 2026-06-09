use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::types::{BlindingFactor, Commitment, RangeProof, VerifyRangeProofRewindResult};
use crate::codec::{decode, encode};

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
    let result = session.crypto_call(
        "verify_range_proof_rewind",
        verify_range_proof_rewind_params(nonce, commit, proof),
    )?;
    decode("verify_range_proof_rewind", result)
}

fn verify_range_proof_rewind_params(nonce: Value, commit: Value, proof: Value) -> Value {
    json!([nonce, commit, proof])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_range_proof_rewind_params_pass_nonce_commit_proof() {
        assert_eq!(
            verify_range_proof_rewind_params(json!("aa"), json!("bb"), json!("cc")),
            json!(["aa", "bb", "cc"])
        );
    }
}
