use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::types::{Commitment, RangeProof, VerifyRangeResult};
use crate::codec::{decode, encode};

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
    let result = session.crypto_call("verify_range", verify_range_params(commit, proof))?;
    decode("verify_range", result)
}

fn verify_range_params(commit: Value, proof: Value) -> Value {
    json!([commit, proof])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_range_params_pass_commit_then_proof() {
        assert_eq!(
            verify_range_params(json!("aa"), json!("bb")),
            json!(["aa", "bb"])
        );
    }
}
