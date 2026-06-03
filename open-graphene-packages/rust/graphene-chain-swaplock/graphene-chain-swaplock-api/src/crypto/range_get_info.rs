use open_graphene_transport::{GrapheneSession, crypto_api};

use crate::SwaplockApiError;

use super::error::{decode, encode};
use super::types::{RangeProof, RangeProofInfo};

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
    let result = crypto_api::range_get_info(session, proof)?;
    decode("range_get_info", result)
}
