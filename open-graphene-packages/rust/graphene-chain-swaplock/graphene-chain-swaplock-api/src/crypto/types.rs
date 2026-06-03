//! Domain types for the Swaplock `crypto` API.
//!
//! On the wire, Graphene (via `fc`) encodes blinding factors, Pedersen
//! commitments and range proofs as **lowercase hex strings**, not byte arrays.
//! These newtypes own that encoding: they (de)serialize as hex, so callers work
//! with typed values and can never accidentally pass a proof where a commitment
//! is expected.

use serde::{Deserialize, Serialize};

use crate::SwaplockApiError;

macro_rules! hex_newtype {
    ($(#[$meta:meta])* $name:ident, $kind:literal) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(Vec<u8>);

        impl $name {
            /// Wrap raw bytes.
            pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
                Self(bytes.into())
            }

            /// Parse a hex string (with or without leading/trailing whitespace).
            pub fn from_hex(hex: &str) -> Result<Self, SwaplockApiError> {
                let bytes = hex::decode(hex.trim()).map_err(|error| {
                    SwaplockApiError::InvalidHex {
                        kind: $kind,
                        message: error.to_string(),
                    }
                })?;
                Ok(Self(bytes))
            }

            /// Borrow the raw bytes.
            pub fn as_bytes(&self) -> &[u8] {
                &self.0
            }

            /// Consume into raw bytes.
            pub fn into_bytes(self) -> Vec<u8> {
                self.0
            }

            /// Lowercase hex encoding (the on-wire form).
            pub fn to_hex(&self) -> String {
                hex::encode(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&hex::encode(&self.0))
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let hex = String::deserialize(deserializer)?;
                let bytes = hex::decode(hex.trim()).map_err(serde::de::Error::custom)?;
                Ok(Self(bytes))
            }
        }
    };
}

hex_newtype!(
    /// A 32-byte secret scalar (`fc::sha256`) that hides a value inside a commitment.
    BlindingFactor,
    "blinding factor"
);

hex_newtype!(
    /// A 33-byte Pedersen commitment: `commit = blind·G + value·H`.
    Commitment,
    "commitment"
);

hex_newtype!(
    /// A Borromean range proof asserting a committed value lies in a range.
    RangeProof,
    "range proof"
);

/// Result of `verify_range`: whether the proof is valid and the value bounds it proves.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct VerifyRangeResult {
    pub success: bool,
    pub min_val: u64,
    pub max_val: u64,
}

/// Result of `verify_range_proof_rewind`: range bounds plus the recovered value,
/// blinding factor and message when the proof rewinds successfully.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct VerifyRangeProofRewindResult {
    pub success: bool,
    pub min_val: u64,
    pub max_val: u64,
    pub value_out: u64,
    pub blind_out: BlindingFactor,
    pub message_out: String,
}

/// Result of `range_get_info`: the proof's exponent, mantissa and range bounds.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RangeProofInfo {
    pub exp: i32,
    pub mantissa: i32,
    pub min_value: u64,
    pub max_value: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn commitment_serializes_as_lowercase_hex_string() {
        let commitment = Commitment::from_bytes(vec![0x0a, 0xff]);
        assert_eq!(serde_json::to_value(&commitment).unwrap(), json!("0aff"));
    }

    #[test]
    fn commitment_round_trips_through_hex() {
        let commitment = Commitment::from_hex("0aff").unwrap();
        assert_eq!(commitment.as_bytes(), &[0x0a, 0xff]);
        assert_eq!(commitment.to_hex(), "0aff");
    }

    #[test]
    fn from_hex_reports_invalid_input_with_kind() {
        let error = BlindingFactor::from_hex("zz").unwrap_err();
        assert!(matches!(
            error,
            SwaplockApiError::InvalidHex {
                kind: "blinding factor",
                ..
            }
        ));
    }

    #[test]
    fn verify_range_result_deserializes_from_node_shape() {
        let result: VerifyRangeResult =
            serde_json::from_value(json!({"success": true, "min_val": 0, "max_val": 1000}))
                .unwrap();
        assert!(result.success);
        assert_eq!(result.max_val, 1000);
    }

    #[test]
    fn rewind_result_decodes_blind_out_as_hex() {
        let result: VerifyRangeProofRewindResult = serde_json::from_value(json!({
            "success": true,
            "min_val": 0,
            "max_val": 500,
            "value_out": 250,
            "blind_out": "1122",
            "message_out": ""
        }))
        .unwrap();
        assert_eq!(result.value_out, 250);
        assert_eq!(result.blind_out.as_bytes(), &[0x11, 0x22]);
    }
}
