use open_graphene_transport::{GrapheneSession, crypto_api};

use crate::SwaplockApiError;

use super::error::{decode, encode};
use super::types::{BlindingFactor, Commitment};

/// Builder for `crypto.blind`: turn a blinding factor + value into a Pedersen commitment.
pub struct BlindRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) blinding_factor: BlindingFactor,
    pub(super) value: u64,
}

impl BlindRequest<'_> {
    pub async fn get(self) -> Result<Commitment, SwaplockApiError> {
        blind(self.session, self.blinding_factor, self.value).await
    }
}

pub(super) async fn blind(
    session: &mut GrapheneSession,
    blinding_factor: BlindingFactor,
    value: u64,
) -> Result<Commitment, SwaplockApiError> {
    let factor = encode("blind", &blinding_factor)?;
    let result = crypto_api::blind(session, factor, value)?;
    decode("blind", result)
}
