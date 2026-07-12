use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::types::{RangeProof, RangeProofInfo};
use crate::codec::{decode, encode};

/// Builder for `crypto.range_get_info`: read a proof's exponent, mantissa and
/// the value range it covers — without verifying it against a commitment.
pub struct RangeGetInfoRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) proof: RangeProof,
}

impl RangeGetInfoRequest<'_> {
    pub async fn get(self) -> Result<RangeProofInfo, SwaplockApiError> {
        range_get_info(self.session, self.proof).await
    }
}

pub(super) async fn range_get_info(
    session: &mut GrapheneSession,
    proof: RangeProof,
) -> Result<RangeProofInfo, SwaplockApiError> {
    let proof = encode("range_get_info", &proof)?;
    let result = session
        .crypto_call("range_get_info", range_get_info_params(proof))
        .await?;
    decode("range_get_info", result)
}

fn range_get_info_params(proof: Value) -> Value {
    json!([proof])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_get_info_params_wrap_proof() {
        assert_eq!(range_get_info_params(json!("ff")), json!(["ff"]));
    }
}
