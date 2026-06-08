//! Shared JSON (de)serialization helpers for typed RPC arguments and results.
//!
//! Every typed API module turns domain values into wire JSON and back the same way,
//! mapping any (de)serialization failure to [`SwaplockApiError::UnexpectedResponse`].

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::SwaplockApiError;

/// Encode a typed argument into its JSON wire form.
pub(crate) fn encode<T: Serialize>(
    method: &'static str,
    value: &T,
) -> Result<Value, SwaplockApiError> {
    serde_json::to_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}

/// Decode a node response into the expected typed result.
pub(crate) fn decode<T: DeserializeOwned>(
    method: &'static str,
    value: Value,
) -> Result<T, SwaplockApiError> {
    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Commitment;
    use serde_json::json;

    #[test]
    fn encode_renders_commitment_as_hex() {
        let commitment = Commitment::from_bytes(vec![0xab, 0xcd]);
        assert_eq!(encode("blind", &commitment).unwrap(), json!("abcd"));
    }

    #[test]
    fn decode_reports_unexpected_shape() {
        let error = decode::<bool>("verify_sum", json!("nope")).unwrap_err();
        assert!(matches!(
            error,
            SwaplockApiError::UnexpectedResponse {
                method: "verify_sum",
                ..
            }
        ));
    }
}
