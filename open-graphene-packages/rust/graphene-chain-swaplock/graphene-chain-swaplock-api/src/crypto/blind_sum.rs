use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::types::BlindingFactor;
use crate::codec::{decode, encode};

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
    let result = session.crypto_call("blind_sum", blind_sum_params(blinds, non_neg))?;
    decode("blind_sum", result)
}

fn blind_sum_params(blinds_in: Value, non_neg: u32) -> Value {
    json!([blinds_in, non_neg])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blind_sum_params_pass_factor_array_then_non_neg() {
        assert_eq!(
            blind_sum_params(json!(["1111", "2222"]), 2),
            json!([["1111", "2222"], 2])
        );
    }
}
