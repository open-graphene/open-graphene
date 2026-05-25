use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AmountError {
    #[error("amount must not be empty")]
    Empty,
    #[error("amount must not be negative")]
    Negative,
    #[error("amount must be a decimal number")]
    InvalidDecimal,
    #[error("amount supports at most {precision} decimal places")]
    TooManyDecimalPlaces { precision: u8 },
    #[error("amount exceeds i64 range")]
    OutOfRange,
}

pub fn decimal_to_raw_amount(value: &str, precision: u8) -> Result<i64, AmountError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AmountError::Empty);
    }
    if value.starts_with('-') {
        return Err(AmountError::Negative);
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() > 2
        || parts
            .iter()
            .any(|part| !part.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(AmountError::InvalidDecimal);
    }
    let integer = if parts[0].is_empty() { "0" } else { parts[0] };
    let mut fractional = parts.get(1).copied().unwrap_or("").to_string();
    while fractional.ends_with('0') {
        fractional.pop();
    }
    if fractional.len() > precision as usize {
        return Err(AmountError::TooManyDecimalPlaces { precision });
    }
    while fractional.len() < precision as usize {
        fractional.push('0');
    }
    let raw = format!("{integer}{fractional}");
    let raw = raw.trim_start_matches('0');
    if raw.is_empty() {
        Ok(0)
    } else {
        raw.parse().map_err(|_| AmountError::OutOfRange)
    }
}

pub fn format_raw_amount(amount: i64, precision: u8) -> String {
    let sign = if amount < 0 { "-" } else { "" };
    let amount = amount.unsigned_abs().to_string();
    if precision == 0 {
        return format!("{sign}{amount}");
    }
    let precision = precision as usize;
    if amount.len() <= precision {
        let zeroes = "0".repeat(precision - amount.len());
        return format!("{sign}0.{zeroes}{amount}");
    }
    let split = amount.len() - precision;
    format!("{sign}{}.{}", &amount[..split], &amount[split..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_amount_converts_to_raw_amount_using_asset_precision() {
        assert_eq!(decimal_to_raw_amount("1", 5).unwrap(), 100_000);
        assert_eq!(decimal_to_raw_amount("1.23", 5).unwrap(), 123_000);
        assert_eq!(decimal_to_raw_amount("0.00001", 5).unwrap(), 1);
        assert_eq!(decimal_to_raw_amount("1.23000", 5).unwrap(), 123_000);
        assert_eq!(
            decimal_to_raw_amount("0.000001", 5),
            Err(AmountError::TooManyDecimalPlaces { precision: 5 })
        );
    }

    #[test]
    fn decimal_amount_rejects_invalid_values() {
        assert_eq!(decimal_to_raw_amount("", 5), Err(AmountError::Empty));
        assert_eq!(decimal_to_raw_amount("-1", 5), Err(AmountError::Negative));
        assert_eq!(
            decimal_to_raw_amount("1.2.3", 5),
            Err(AmountError::InvalidDecimal)
        );
        assert_eq!(
            decimal_to_raw_amount("abc", 5),
            Err(AmountError::InvalidDecimal)
        );
    }

    #[test]
    fn raw_amount_formats_using_asset_precision() {
        assert_eq!(format_raw_amount(100_000, 5), "1.00000");
        assert_eq!(format_raw_amount(123_000, 5), "1.23000");
        assert_eq!(format_raw_amount(1, 5), "0.00001");
        assert_eq!(format_raw_amount(0, 5), "0.00000");
        assert_eq!(format_raw_amount(-123_000, 5), "-1.23000");
        assert_eq!(format_raw_amount(42, 0), "42");
    }
}
