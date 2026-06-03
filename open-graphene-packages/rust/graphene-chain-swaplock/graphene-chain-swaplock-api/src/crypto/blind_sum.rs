use open_graphene_transport::{GrapheneSession, crypto_api};

use crate::SwaplockApiError;

use super::error::{decode, encode};
use super::types::BlindingFactor;

/// Builder for `crypto.blind_sum`: sum blinding factors (first `non_neg` positive,
/// the rest negative) into one factor. Used to balance the blinds of a transaction.
pub struct BlindSumRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) blinds_in: Vec<BlindingFactor>,
    pub(super) non_neg: u32,
}

impl BlindSumRequest<'_> {
    pub async fn get(self) -> Result<BlindingFactor, SwaplockApiError> {
        blind_sum(self.session, self.blinds_in, self.non_neg).await
    }
}

pub(super) async fn blind_sum(
    session: &mut GrapheneSession,
    blinds_in: Vec<BlindingFactor>,
    non_neg: u32,
) -> Result<BlindingFactor, SwaplockApiError> {
    let blinds = encode("blind_sum", &blinds_in)?;
    let result = crypto_api::blind_sum(session, blinds, non_neg)?;
    decode("blind_sum", result)
}
