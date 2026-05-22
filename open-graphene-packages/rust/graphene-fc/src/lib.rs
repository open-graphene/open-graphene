use ripemd::Digest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FcSerializeError {
    InvalidProtocolObjectId {
        value: String,
        expected_space: Option<u32>,
        expected_type: Option<u32>,
    },
    InvalidPublicKey {
        value: String,
        expected_prefix: Option<String>,
        reason: &'static str,
    },
    UnsupportedVariant {
        variant: &'static str,
    },
    UnsupportedValue {
        type_name: &'static str,
        reason: &'static str,
    },
}

impl std::fmt::Display for FcSerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProtocolObjectId {
                value,
                expected_space,
                expected_type,
            } => write!(
                f,
                "invalid protocol object id `{value}` for expected space {expected_space:?} and type {expected_type:?}"
            ),
            Self::InvalidPublicKey {
                value,
                expected_prefix,
                reason,
            } => write!(
                f,
                "invalid public key `{value}` for expected prefix {expected_prefix:?}: {reason}"
            ),
            Self::UnsupportedVariant { variant } => {
                write!(
                    f,
                    "FC serialization is not implemented for variant `{variant}`"
                )
            }
            Self::UnsupportedValue { type_name, reason } => {
                write!(
                    f,
                    "FC serialization is not implemented for `{type_name}`: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for FcSerializeError {}

pub type Result<T> = std::result::Result<T, FcSerializeError>;

pub trait FcSerialize {
    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()>;

    fn to_fc_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        self.fc_serialize(&mut out)?;
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolObjectIdParts {
    pub space: u32,
    pub type_id: u32,
    pub instance: u64,
}

pub fn write_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push(((value as u8) & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

pub fn parse_protocol_object_id(
    value: &str,
    expected_space: Option<u32>,
    expected_type: Option<u32>,
) -> Result<ProtocolObjectIdParts> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(invalid_protocol_object_id(
            value,
            expected_space,
            expected_type,
        ));
    }

    let space = parts[0]
        .parse::<u32>()
        .map_err(|_| invalid_protocol_object_id(value, expected_space, expected_type))?;
    let type_id = parts[1]
        .parse::<u32>()
        .map_err(|_| invalid_protocol_object_id(value, expected_space, expected_type))?;
    let instance = parts[2]
        .parse::<u64>()
        .map_err(|_| invalid_protocol_object_id(value, expected_space, expected_type))?;

    if expected_space.is_some_and(|expected| expected != space)
        || expected_type.is_some_and(|expected| expected != type_id)
    {
        return Err(invalid_protocol_object_id(
            value,
            expected_space,
            expected_type,
        ));
    }

    Ok(ProtocolObjectIdParts {
        space,
        type_id,
        instance,
    })
}

pub fn write_protocol_object_id(
    value: &str,
    expected_space: Option<u32>,
    expected_type: Option<u32>,
    out: &mut Vec<u8>,
) -> Result<()> {
    let parts = parse_protocol_object_id(value, expected_space, expected_type)?;
    write_varint(parts.instance, out);
    Ok(())
}

pub fn write_public_key(
    value: &str,
    expected_prefix: Option<&str>,
    out: &mut Vec<u8>,
) -> Result<()> {
    let payload = match expected_prefix {
        Some(prefix) => value.strip_prefix(prefix).ok_or_else(|| invalid_public_key(
            value,
            expected_prefix,
            "missing expected chain prefix",
        ))?,
        None => value,
    };

    let decoded = bs58::decode(payload).into_vec().map_err(|_| {
        invalid_public_key(value, expected_prefix, "base58 payload is invalid")
    })?;

    if decoded.len() != 37 {
        return Err(invalid_public_key(
            value,
            expected_prefix,
            "expected 33 byte compressed key plus 4 byte checksum",
        ));
    }

    let (key, checksum) = decoded.split_at(33);
    if !matches!(key.first(), Some(0x02 | 0x03)) {
        return Err(invalid_public_key(
            value,
            expected_prefix,
            "invalid compressed public key prefix",
        ));
    }

    let digest = ripemd::Ripemd160::digest(key);
    if checksum != &digest[..4] {
        return Err(invalid_public_key(
            value,
            expected_prefix,
            "RIPEMD160 checksum mismatch",
        ));
    }

    out.extend_from_slice(key);
    Ok(())
}

fn invalid_protocol_object_id(
    value: &str,
    expected_space: Option<u32>,
    expected_type: Option<u32>,
) -> FcSerializeError {
    FcSerializeError::InvalidProtocolObjectId {
        value: value.to_string(),
        expected_space,
        expected_type,
    }
}

fn invalid_public_key(
    value: &str,
    expected_prefix: Option<&str>,
    reason: &'static str,
) -> FcSerializeError {
    FcSerializeError::InvalidPublicKey {
        value: value.to_string(),
        expected_prefix: expected_prefix.map(str::to_string),
        reason,
    }
}

impl FcSerialize for () {
    fn fc_serialize(&self, _out: &mut Vec<u8>) -> Result<()> {
        Ok(())
    }
}

impl FcSerialize for bool {
    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {
        out.push(u8::from(*self));
        Ok(())
    }
}

impl FcSerialize for u8 {
    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {
        out.push(*self);
        Ok(())
    }
}

macro_rules! impl_fc_fixed_width_integer {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl FcSerialize for $ty {
                fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {
                    out.extend_from_slice(&self.to_le_bytes());
                    Ok(())
                }
            }
        )+
    };
}

impl_fc_fixed_width_integer!(u16, u32, u64, i32, i64);

impl FcSerialize for str {
    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {
        write_varint(self.len() as u64, out);
        out.extend_from_slice(self.as_bytes());
        Ok(())
    }
}

impl FcSerialize for String {
    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {
        self.as_str().fc_serialize(out)
    }
}

impl<T: FcSerialize> FcSerialize for Option<T> {
    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {
        match self {
            Some(value) => {
                out.push(1);
                value.fc_serialize(out)
            }
            None => {
                out.push(0);
                Ok(())
            }
        }
    }
}

impl<T: FcSerialize> FcSerialize for Vec<T> {
    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {
        write_varint(self.len() as u64, out);
        for value in self {
            value.fc_serialize(out)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_writes_single_and_multi_byte_values() {
        let mut out = Vec::new();
        write_varint(0, &mut out);
        assert_eq!(out, [0]);

        let mut out = Vec::new();
        write_varint(128, &mut out);
        assert_eq!(out, [0x80, 0x01]);
    }

    #[test]
    fn bool_serializes_as_single_fc_byte() {
        assert_eq!(false.to_fc_bytes().unwrap(), [0]);
        assert_eq!(true.to_fc_bytes().unwrap(), [1]);
    }

    #[test]
    fn fixed_width_integers_serialize_little_endian() {
        assert_eq!(0xabu8.to_fc_bytes().unwrap(), [0xab]);
        assert_eq!(0x1234u16.to_fc_bytes().unwrap(), [0x34, 0x12]);
        assert_eq!(0x1234_5678u32.to_fc_bytes().unwrap(), [0x78, 0x56, 0x34, 0x12]);
        assert_eq!(
            0x0123_4567_89ab_cdefu64.to_fc_bytes().unwrap(),
            [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]
        );
        assert_eq!((-2i32).to_fc_bytes().unwrap(), [0xfe, 0xff, 0xff, 0xff]);
        assert_eq!((-2i64).to_fc_bytes().unwrap(), [0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn strings_serialize_as_varint_length_prefixed_utf8() {
        assert_eq!("".to_fc_bytes().unwrap(), [0]);
        assert_eq!("abc".to_fc_bytes().unwrap(), [3, b'a', b'b', b'c']);
        assert_eq!(String::from("ż").to_fc_bytes().unwrap(), [2, 0xc5, 0xbc]);
    }

    #[test]
    fn public_key_writes_compressed_key_bytes_and_validates_checksum() {
        let key = "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV";
        let mut out = Vec::new();
        write_public_key(key, Some("BTS"), &mut out).unwrap();

        assert_eq!(out.len(), 33);
        assert_eq!(out[0], 0x02);
        assert_eq!(
            hex_string(&out),
            "02c0ded2bc1f1305fb0faac5e6c03ee3a1924234985427b6167ca569d13df435cf"
        );

        let err = write_public_key(key, Some("GPH"), &mut Vec::new())
            .expect_err("wrong prefix fails");
        assert!(matches!(
            err,
            FcSerializeError::InvalidPublicKey {
                expected_prefix: Some(prefix),
                reason: "missing expected chain prefix",
                ..
            } if prefix == "GPH"
        ));

        let err = write_public_key(
            "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CW",
            Some("BTS"),
            &mut Vec::new(),
        )
        .expect_err("checksum mismatch fails");
        assert!(matches!(
            err,
            FcSerializeError::InvalidPublicKey {
                reason: "RIPEMD160 checksum mismatch",
                ..
            }
        ));
    }

    #[test]
    fn protocol_object_id_validates_expected_space_and_type() {
        let mut out = Vec::new();
        write_protocol_object_id("1.2.345", Some(1), Some(2), &mut out).unwrap();
        assert_eq!(out, [0xd9, 0x02]);

        let err = write_protocol_object_id("1.3.345", Some(1), Some(2), &mut Vec::new())
            .expect_err("wrong type id fails");
        assert!(matches!(
            err,
            FcSerializeError::InvalidProtocolObjectId {
                expected_space: Some(1),
                expected_type: Some(2),
                ..
            }
        ));
    }

    fn hex_string(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
