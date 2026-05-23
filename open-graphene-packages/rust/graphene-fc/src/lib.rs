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
    InvalidTimePointSec {
        value: String,
        reason: &'static str,
    },
    InvalidVoteId {
        value: String,
        reason: &'static str,
    },
    InvalidFixedBytes {
        type_name: &'static str,
        expected_len: usize,
        actual_len: usize,
    },
    InvalidChainId {
        value: String,
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
            Self::InvalidTimePointSec { value, reason } => {
                write!(f, "invalid time_point_sec `{value}`: {reason}")
            }
            Self::InvalidVoteId { value, reason } => {
                write!(f, "invalid vote id `{value}`: {reason}")
            }
            Self::InvalidFixedBytes {
                type_name,
                expected_len,
                actual_len,
            } => write!(
                f,
                "invalid fixed bytes `{type_name}`: expected {expected_len} bytes, got {actual_len}"
            ),
            Self::InvalidChainId { value, reason } => {
                write!(f, "invalid chain id `{value}`: {reason}")
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteIdParts {
    pub type_id: u8,
    pub instance: u32,
    pub content: u32,
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
        Some(prefix) => value.strip_prefix(prefix).ok_or_else(|| {
            invalid_public_key(value, expected_prefix, "missing expected chain prefix")
        })?,
        None => value,
    };

    let decoded = bs58::decode(payload)
        .into_vec()
        .map_err(|_| invalid_public_key(value, expected_prefix, "base58 payload is invalid"))?;

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

pub fn parse_time_point_sec(value: &str) -> Result<u32> {
    if value.len() != "YYYY-MM-DDTHH:MM:SS".len() {
        return Err(invalid_time_point_sec(
            value,
            "expected YYYY-MM-DDTHH:MM:SS",
        ));
    }

    let bytes = value.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err(invalid_time_point_sec(
            value,
            "expected YYYY-MM-DDTHH:MM:SS",
        ));
    }

    let year = parse_fixed_digits(value, 0, 4, "year")?;
    let month = parse_fixed_digits(value, 5, 2, "month")?;
    let day = parse_fixed_digits(value, 8, 2, "day")?;
    let hour = parse_fixed_digits(value, 11, 2, "hour")?;
    let minute = parse_fixed_digits(value, 14, 2, "minute")?;
    let second = parse_fixed_digits(value, 17, 2, "second")?;

    if year < 1970 {
        return Err(invalid_time_point_sec(value, "year is before Unix epoch"));
    }
    if !(1..=12).contains(&month) {
        return Err(invalid_time_point_sec(value, "month out of range"));
    }
    let days_in_month = days_in_month(year, month);
    if day == 0 || day > days_in_month {
        return Err(invalid_time_point_sec(value, "day out of range"));
    }
    if hour > 23 {
        return Err(invalid_time_point_sec(value, "hour out of range"));
    }
    if minute > 59 {
        return Err(invalid_time_point_sec(value, "minute out of range"));
    }
    if second > 59 {
        return Err(invalid_time_point_sec(value, "second out of range"));
    }

    let days = days_since_unix_epoch(year, month, day);
    let seconds = days
        .checked_mul(86_400)
        .and_then(|base| base.checked_add((hour as u64) * 3_600))
        .and_then(|base| base.checked_add((minute as u64) * 60))
        .and_then(|base| base.checked_add(second as u64))
        .ok_or_else(|| invalid_time_point_sec(value, "timestamp is out of u32 range"))?;

    u32::try_from(seconds)
        .map_err(|_| invalid_time_point_sec(value, "timestamp is out of u32 range"))
}

pub fn write_time_point_sec(value: &str, out: &mut Vec<u8>) -> Result<()> {
    let seconds = parse_time_point_sec(value)?;
    out.extend_from_slice(&seconds.to_le_bytes());
    Ok(())
}

pub fn parse_vote_id(value: &str) -> Result<VoteIdParts> {
    let (type_part, instance_part) = value
        .split_once(':')
        .ok_or_else(|| invalid_vote_id(value, "expected type:instance"))?;

    if type_part.is_empty() || instance_part.is_empty() || instance_part.contains(':') {
        return Err(invalid_vote_id(value, "expected type:instance"));
    }
    if !type_part.bytes().all(|byte| byte.is_ascii_digit())
        || !instance_part.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid_vote_id(
            value,
            "type and instance must be decimal integers",
        ));
    }

    let type_id = type_part
        .parse::<u32>()
        .map_err(|_| invalid_vote_id(value, "type is out of range"))?;
    if type_id > u8::MAX as u32 {
        return Err(invalid_vote_id(value, "type exceeds 8 bits"));
    }

    let instance = instance_part
        .parse::<u32>()
        .map_err(|_| invalid_vote_id(value, "instance is out of range"))?;
    if instance >= 0x0100_0000 {
        return Err(invalid_vote_id(value, "instance exceeds 24 bits"));
    }

    let content = (instance << 8) | type_id;
    Ok(VoteIdParts {
        type_id: type_id as u8,
        instance,
        content,
    })
}

pub fn write_vote_id(value: &str, out: &mut Vec<u8>) -> Result<()> {
    let parts = parse_vote_id(value)?;
    out.extend_from_slice(&parts.content.to_le_bytes());
    Ok(())
}

pub fn write_bytes(value: &[u8], out: &mut Vec<u8>) -> Result<()> {
    write_varint(value.len() as u64, out);
    out.extend_from_slice(value);
    Ok(())
}

pub fn write_fixed_bytes(
    value: &[u8],
    expected_len: usize,
    type_name: &'static str,
    out: &mut Vec<u8>,
) -> Result<()> {
    if value.len() != expected_len {
        return Err(FcSerializeError::InvalidFixedBytes {
            type_name,
            expected_len,
            actual_len: value.len(),
        });
    }
    out.extend_from_slice(value);
    Ok(())
}

pub fn decode_chain_id_hex(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 {
        return Err(invalid_chain_id(
            value,
            "chain id must be 64 lowercase hex characters",
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_chain_id(value, "chain id must be lowercase hex"));
    }

    let mut out = [0u8; 32];
    let bytes = value.as_bytes();
    for index in 0..32 {
        let high = decode_lower_hex_nibble(bytes[index * 2]);
        let low = decode_lower_hex_nibble(bytes[index * 2 + 1]);
        out[index] = (high << 4) | low;
    }
    Ok(out)
}

fn decode_lower_hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("decode_chain_id_hex validates lowercase hex before decoding"),
    }
}

fn parse_fixed_digits(
    value: &str,
    start: usize,
    len: usize,
    component: &'static str,
) -> Result<u32> {
    let part = &value[start..start + len];
    if !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid_time_point_sec(value, component));
    }
    part.parse::<u32>()
        .map_err(|_| invalid_time_point_sec(value, component))
}

fn days_since_unix_epoch(year: u32, month: u32, day: u32) -> u64 {
    let mut days = 0u64;
    for current_year in 1970..year {
        days += if is_leap_year(current_year) { 366 } else { 365 };
    }
    for current_month in 1..month {
        days += u64::from(days_in_month(year, current_month));
    }
    days + u64::from(day - 1)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400)
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

fn invalid_time_point_sec(value: &str, reason: &'static str) -> FcSerializeError {
    FcSerializeError::InvalidTimePointSec {
        value: value.to_string(),
        reason,
    }
}

fn invalid_vote_id(value: &str, reason: &'static str) -> FcSerializeError {
    FcSerializeError::InvalidVoteId {
        value: value.to_string(),
        reason,
    }
}

fn invalid_chain_id(value: &str, reason: &'static str) -> FcSerializeError {
    FcSerializeError::InvalidChainId {
        value: value.to_string(),
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
        assert_eq!(
            0x1234_5678u32.to_fc_bytes().unwrap(),
            [0x78, 0x56, 0x34, 0x12]
        );
        assert_eq!(
            0x0123_4567_89ab_cdefu64.to_fc_bytes().unwrap(),
            [0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]
        );
        assert_eq!((-2i32).to_fc_bytes().unwrap(), [0xfe, 0xff, 0xff, 0xff]);
        assert_eq!(
            (-2i64).to_fc_bytes().unwrap(),
            [0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn strings_serialize_as_varint_length_prefixed_utf8() {
        assert_eq!("".to_fc_bytes().unwrap(), [0]);
        assert_eq!("abc".to_fc_bytes().unwrap(), [3, b'a', b'b', b'c']);
        assert_eq!(String::from("ż").to_fc_bytes().unwrap(), [2, 0xc5, 0xbc]);
    }

    #[test]
    fn bytes_write_varint_length_prefixed_raw_bytes() {
        let mut out = Vec::new();
        write_bytes(&[0xab, 0xcd], &mut out).unwrap();
        assert_eq!(out, [2, 0xab, 0xcd]);
    }

    #[test]
    fn fixed_bytes_validate_exact_length_and_write_raw_bytes() {
        let mut out = Vec::new();
        write_fixed_bytes(&[0xab, 0xcd], 2, "test_fixed", &mut out).unwrap();
        assert_eq!(out, [0xab, 0xcd]);

        let err = write_fixed_bytes(&[0xab], 2, "test_fixed", &mut out)
            .expect_err("invalid fixed bytes length fails");
        assert!(matches!(
            err,
            FcSerializeError::InvalidFixedBytes {
                type_name: "test_fixed",
                expected_len: 2,
                actual_len: 1,
            }
        ));
    }

    #[test]
    fn chain_id_hex_decodes_exact_32_byte_lowercase_hex() {
        let decoded =
            decode_chain_id_hex("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
                .expect("decode valid chain id");
        assert_eq!(decoded[0..4], [0x22, 0x67, 0xf6, 0x94]);
        assert_eq!(decoded[28..32], [0x42, 0x23, 0x40, 0x98]);
    }

    #[test]
    fn chain_id_hex_rejects_invalid_length_or_characters() {
        assert!(matches!(
            decode_chain_id_hex("abcd"),
            Err(FcSerializeError::InvalidChainId {
                reason: "chain id must be 64 lowercase hex characters",
                ..
            })
        ));
        assert!(matches!(
            decode_chain_id_hex("2267F694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098"),
            Err(FcSerializeError::InvalidChainId {
                reason: "chain id must be lowercase hex",
                ..
            })
        ));
    }

    #[test]
    fn time_point_sec_writes_u32_seconds_little_endian() {
        let mut out = Vec::new();
        write_time_point_sec("1970-01-01T00:00:00", &mut out).unwrap();
        assert_eq!(out, [0, 0, 0, 0]);

        let mut out = Vec::new();
        write_time_point_sec("1970-01-01T00:00:01", &mut out).unwrap();
        assert_eq!(out, [1, 0, 0, 0]);

        let mut out = Vec::new();
        write_time_point_sec("2020-01-02T03:04:05", &mut out).unwrap();
        assert_eq!(out, [0xa5, 0x5d, 0x0d, 0x5e]);

        let mut out = Vec::new();
        write_time_point_sec("2106-02-07T06:28:15", &mut out).unwrap();
        assert_eq!(out, [0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn time_point_sec_rejects_ambiguous_or_out_of_range_values() {
        for value in [
            "1970-01-01T00:00:00Z",
            "1970-01-01T00:00:00.000",
            "1969-12-31T23:59:59",
            "2023-02-29T00:00:00",
            "2023-01-01T24:00:00",
            "2106-02-07T06:28:16",
            "bad",
        ] {
            assert!(
                matches!(
                    write_time_point_sec(value, &mut Vec::new()),
                    Err(FcSerializeError::InvalidTimePointSec { .. })
                ),
                "{value} should fail"
            );
        }
    }

    #[test]
    fn vote_id_writes_packed_u32_little_endian() {
        let mut out = Vec::new();
        write_vote_id("0:0", &mut out).unwrap();
        assert_eq!(out, [0, 0, 0, 0]);

        let mut out = Vec::new();
        write_vote_id("1:5", &mut out).unwrap();
        assert_eq!(out, [1, 5, 0, 0]);

        let mut out = Vec::new();
        write_vote_id("2:16777215", &mut out).unwrap();
        assert_eq!(out, [2, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn vote_id_rejects_invalid_or_out_of_range_values() {
        for value in [
            "",
            "1",
            "1:",
            ":5",
            "1:5:7",
            "x:5",
            "1:x",
            "256:0",
            "1:16777216",
        ] {
            assert!(
                matches!(
                    write_vote_id(value, &mut Vec::new()),
                    Err(FcSerializeError::InvalidVoteId { .. })
                ),
                "{value} should fail"
            );
        }
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

        let err =
            write_public_key(key, Some("GPH"), &mut Vec::new()).expect_err("wrong prefix fails");
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
