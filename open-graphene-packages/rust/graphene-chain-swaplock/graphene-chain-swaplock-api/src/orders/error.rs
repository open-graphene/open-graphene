use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::SwaplockApiError;

/// Encode a typed argument into its JSON wire form.
pub(super) fn encode<T: Serialize>(
    method: &'static str,
    value: &T,
) -> Result<Value, SwaplockApiError> {
    serde_json::to_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}

/// Decode a node response into the expected typed result.
pub(super) fn decode<T: DeserializeOwned>(
    method: &'static str,
    value: Value,
) -> Result<T, SwaplockApiError> {
    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method,
        message: error.to_string(),
    })
}
