//! Shared serde helpers for decoding Graphene wire values that are typed the same
//! way across every chain.

use serde::Deserialize;

/// Deserialize an `i64` that may arrive as a JSON number or a decimal string.
///
/// Graphene's `share_type` (signed 64-bit) is serialized as a JSON number for small
/// magnitudes and as a decimal string for large ones; accept both. This is shared
/// typing common to every chain, so it lives in core rather than being re-implemented
/// per API module.
pub fn deserialize_i64_from_number_or_decimal_string<'de, D>(
    deserializer: D,
) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumberOrString {
        Number(i64),
        Text(String),
    }

    match NumberOrString::deserialize(deserializer)? {
        NumberOrString::Number(value) => Ok(value),
        NumberOrString::Text(text) => text.trim().parse().map_err(serde::de::Error::custom),
    }
}

/// Deserialize fixed-length bytes that arrive either as a hex string or a JSON byte array.
///
/// Graphene serializes fixed-byte fields (object ids, keys, hashes) as lowercase hex on the
/// wire but as a `[u8]` array in some contexts; accept both and enforce `expected_len`. Shared
/// typing common to every chain — the generated bindings re-export this rather than re-emitting
/// it per chain.
pub fn deserialize_fixed_bytes_from_hex_string_or_byte_array<'de, D>(
    deserializer: D,
    expected_len: usize,
) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let bytes = match value {
        serde_json::Value::String(value) => {
            decode_hex_bytes(&value).map_err(serde::de::Error::custom)?
        }
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(|value| match value {
                serde_json::Value::Number(number) => number
                    .as_u64()
                    .and_then(|value| u8::try_from(value).ok())
                    .ok_or_else(|| {
                        serde::de::Error::custom(format!(
                            "expected byte value 0..255, got {number}"
                        ))
                    }),
                other => Err(serde::de::Error::custom(format!(
                    "expected byte value, got {other}"
                ))),
            })
            .collect::<Result<Vec<u8>, D::Error>>()?,
        other => {
            return Err(serde::de::Error::custom(format!(
                "expected fixed bytes as hex string or byte array, got {other}"
            )));
        }
    };
    if bytes.len() != expected_len {
        return Err(serde::de::Error::custom(format!(
            "expected {expected_len} fixed bytes, got {}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

fn decode_hex_bytes(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("hex string has odd length".to_string());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16).map_err(|error| error.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(deserialize_with = "deserialize_i64_from_number_or_decimal_string")]
        value: i64,
    }

    #[test]
    fn decodes_from_json_number() {
        let wrapper: Wrapper = serde_json::from_str(r#"{"value": 5000}"#).unwrap();
        assert_eq!(wrapper.value, 5000);
    }

    #[test]
    fn decodes_from_decimal_string() {
        let wrapper: Wrapper = serde_json::from_str(r#"{"value": "9223372036854775807"}"#).unwrap();
        assert_eq!(wrapper.value, i64::MAX);
    }

    #[test]
    fn rejects_non_numeric_string() {
        let result: Result<Wrapper, _> = serde_json::from_str(r#"{"value": "nope"}"#);
        assert!(result.is_err());
    }
}
