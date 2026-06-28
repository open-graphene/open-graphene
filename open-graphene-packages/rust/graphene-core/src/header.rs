use std::time::Duration;

use thiserror::Error;
use time::PrimitiveDateTime;
use time::format_description;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadBlock {
    pub number: u64,
    pub id: String,
    pub time: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionHeader {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: String,
}

#[derive(Debug, Error)]
pub enum HeaderError {
    #[error("block id hex value has odd length")]
    OddHexLength,
    #[error("block id contains invalid hex")]
    InvalidHex(#[from] std::num::ParseIntError),
    #[error("block id must contain at least 8 bytes")]
    BlockIdTooShort,
    #[error("time format error: {0}")]
    TimeFormat(#[from] time::error::Format),
    #[error("time parse error: {0}")]
    TimeParse(#[from] time::error::Parse),
    #[error("time format description error: {0}")]
    TimeFormatDescription(#[from] time::error::InvalidFormatDescription),
    #[error("expiration offset is out of range")]
    ExpirationOffsetOutOfRange,
}

pub fn transaction_header_from_head(
    head: &HeadBlock,
    expiration_offset: Duration,
) -> Result<TransactionHeader, HeaderError> {
    Ok(TransactionHeader {
        ref_block_num: (head.number & 0xffff) as u16,
        ref_block_prefix: ref_block_prefix_from_block_id(&head.id)?,
        expiration: expiration_from_head_time(&head.time, expiration_offset)?,
    })
}

pub fn ref_block_prefix_from_block_id(block_id: &str) -> Result<u32, HeaderError> {
    let bytes = decode_hex(block_id)?;
    if bytes.len() < 8 {
        return Err(HeaderError::BlockIdTooShort);
    }
    Ok(u32::from_le_bytes(
        bytes[4..8]
            .try_into()
            .expect("slice length was checked above"),
    ))
}

pub fn expiration_from_head_time(head_time: &str, offset: Duration) -> Result<String, HeaderError> {
    let offset =
        time::Duration::try_from(offset).map_err(|_| HeaderError::ExpirationOffsetOutOfRange)?;
    let format = format_description::parse_borrowed::<2>(
        "[year]-[month]-[day]T[hour]:[minute]:[second]",
    )?;
    let parsed = PrimitiveDateTime::parse(head_time, &format)?;
    let expiration = parsed + offset;
    Ok(expiration.format(&format)?)
}

fn decode_hex(value: &str) -> Result<Vec<u8>, HeaderError> {
    if !value.len().is_multiple_of(2) {
        return Err(HeaderError::OddHexLength);
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).map_err(HeaderError::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_derives_ref_block_fields_and_expiration() {
        let header = transaction_header_from_head(
            &HeadBlock {
                number: 609_782,
                id: "00094df644fe617490ae116a0100400d03000000".to_string(),
                time: "2026-05-25T12:00:00".to_string(),
            },
            Duration::from_secs(60),
        )
        .unwrap();

        assert_eq!(header.ref_block_num, 19_958);
        assert_eq!(header.ref_block_prefix, 1_952_579_140);
        assert_eq!(header.expiration, "2026-05-25T12:01:00");
    }

    #[test]
    fn block_prefix_rejects_invalid_block_ids() {
        assert!(matches!(
            ref_block_prefix_from_block_id("abc"),
            Err(HeaderError::OddHexLength)
        ));
        assert!(matches!(
            ref_block_prefix_from_block_id("zz"),
            Err(HeaderError::InvalidHex(_))
        ));
        assert!(matches!(
            ref_block_prefix_from_block_id("00010203"),
            Err(HeaderError::BlockIdTooShort)
        ));
    }
}
