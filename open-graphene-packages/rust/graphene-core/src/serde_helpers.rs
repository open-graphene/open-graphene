//! Shared serde helpers for decoding Graphene wire values that are typed the same
//! way across every chain.

use serde::Deserialize;

/// Deserialize an `i64` that may arrive as a JSON number or a decimal string.
///
/// Graphene's `share_type` (signed 64-bit) is serialized as a JSON number for small
/// magnitudes and as a decimal string for large ones; accept both. "Large" starts just
/// above `u32::MAX` - see the note on the unsigned helper below. This is shared
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

/// Deserialize a `u64` that arrives either as a JSON number or as a decimal string.
///
/// fc renders large unsigned integers as strings, so that a consumer reading JSON with doubles
/// cannot silently round them. The switch happens **above `u32::MAX`, not at 2^53**: measured
/// against a live node, `2^32 + 1` already comes back quoted. Which side of that line a value
/// falls on depends on the value, not the field, so any u64 can arrive either way - `symbol3`
/// in the asset creation fee is a string on one chain and a number on another purely because of
/// what it was set to.
///
/// Assuming the higher threshold is a trap worth naming: it makes anything between 2^32 and 2^53
/// look safe to read as a plain number, and it is not.
pub fn deserialize_u64_from_number_or_decimal_string<'de, D>(
    deserializer: D,
) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumberOrString {
        Number(u64),
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

/// Deserialize variable-length bytes that arrive either as a hex string or a JSON byte array.
///
/// Like [`deserialize_fixed_bytes_from_hex_string_or_byte_array`] but without a length check, for
/// `bytes`/`vector<char>` fields (HTLC preimages, memo messages, custom data).
pub fn deserialize_bytes_from_hex_string_or_byte_array<'de, D>(
    deserializer: D,
) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(value) => {
            decode_hex_bytes(&value).map_err(serde::de::Error::custom)
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
            .collect::<Result<Vec<u8>, D::Error>>(),
        other => Err(serde::de::Error::custom(format!(
            "expected bytes as hex string or byte array, got {other}"
        ))),
    }
}

/// Serialize bytes as the lowercase hex string Graphene expects on the wire.
///
/// The serialize counterpart to the hex byte decoders; the generated bindings use it for
/// `bytes`/`fixed_bytes` fields and hash payloads so broadcasts match the node's JSON.
pub fn serialize_bytes_as_hex<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&bytes_to_hex(bytes))
}

/// Lowercase hex encoding of `bytes`.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
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

    #[test]
    fn encodes_bytes_as_lowercase_hex() {
        assert_eq!(bytes_to_hex(&[0x00, 0x0a, 0xff]), "000aff");
        assert_eq!(bytes_to_hex(&[]), "");
    }

    #[test]
    fn decodes_variable_bytes_from_hex_or_array() {
        let from_hex: Vec<u8> =
            deserialize_bytes_from_hex_string_or_byte_array(serde_json::json!("000aff")).unwrap();
        assert_eq!(from_hex, vec![0x00, 0x0a, 0xff]);
        let from_array: Vec<u8> =
            deserialize_bytes_from_hex_string_or_byte_array(serde_json::json!([0, 10, 255]))
                .unwrap();
        assert_eq!(from_array, vec![0x00, 0x0a, 0xff]);
    }
}
