#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FcSerializeError {
    InvalidProtocolObjectId {
        value: String,
        expected_space: Option<u32>,
        expected_type: Option<u32>,
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
}
