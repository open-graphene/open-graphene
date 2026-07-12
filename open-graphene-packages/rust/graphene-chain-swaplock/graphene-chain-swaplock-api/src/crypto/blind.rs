use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::types::{BlindingFactor, Commitment};
use crate::codec::{decode, encode};

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
    let result = session
        .crypto_call("blind", blind_params(factor, value))
        .await?;
    decode("blind", result)
}

fn blind_params(blinding_factor: Value, value: u64) -> Value {
    json!([blinding_factor, value])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blind_params_pass_hex_factor_then_value() {
        assert_eq!(blind_params(json!("0a0b"), 1_000), json!(["0a0b", 1_000]));
    }
}
