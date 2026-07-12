//! Chain addresses: the short `RIPEMD160(SHA512(public_key))` fingerprint of a public key.
//!
//! A verbatim port of bitsharesjs `PublicKey.toAddressString`. Addresses are rarely needed on
//! modern Graphene chains (authorities name keys, not addresses), but the type is here for parity
//! and for the legacy `address_auths` slot.

use ripemd::{Digest, Ripemd160};
use sha2::Sha512;

use crate::{FcSerializeError, PublicKey, Result};

/// A 20-byte chain address derived from a public key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address([u8; 20]);

impl Address {
    /// Derive the address of `public_key`: `RIPEMD160(SHA512(compressed_pubkey))`.
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        let sha = Sha512::digest(public_key.as_bytes());
        let mut addy = [0u8; 20];
        addy.copy_from_slice(&Ripemd160::digest(sha));
        Self(addy)
    }

    /// Wrap 20 raw address bytes.
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Parse a chain address string. Pass the chain `prefix` (e.g. `"BTS"`) to require it, or `None`
    /// to accept whatever prefix the string carries.
    pub fn from_string(value: &str, prefix: Option<&str>) -> Result<Self> {
        let invalid = |reason: &'static str| FcSerializeError::InvalidPublicKey {
            value: value.to_string(),
            expected_prefix: prefix.map(str::to_string),
            reason,
        };

        let body = match prefix {
            Some(prefix) => value
                .strip_prefix(prefix)
                .ok_or_else(|| invalid("address prefix mismatch"))?,
            None => strip_leading_letters(value),
        };

        let payload = bs58::decode(body)
            .into_vec()
            .map_err(|_| invalid("address is not valid base58"))?;
        if payload.len() != 24 {
            return Err(invalid("address has the wrong length"));
        }
        let (addy, checksum) = payload.split_at(20);
        if Ripemd160::digest(addy)[..4] != checksum[..4] {
            return Err(invalid("RIPEMD160 checksum mismatch"));
        }
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(addy);
        Ok(Self(bytes))
    }

    /// Render as the chain address string (e.g. `BTSabc...`) under `prefix`.
    pub fn to_prefixed_string(&self, prefix: &str) -> String {
        let checksum = Ripemd160::digest(self.0);
        let mut payload = Vec::with_capacity(24);
        payload.extend_from_slice(&self.0);
        payload.extend_from_slice(&checksum[..4]);
        format!("{prefix}{}", bs58::encode(payload).into_string())
    }

    /// The raw 20 address bytes.
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

/// Drop the leading alphabetic prefix (the chain symbol) from an address string when no explicit
/// prefix was given, leaving the base58 body.
fn strip_leading_letters(value: &str) -> &str {
    let end = value
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(value.len());
    &value[end..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrivateKey;

    fn sample_key() -> PublicKey {
        PrivateKey::from_seed(b"open-graphene example seed")
            .unwrap()
            .to_public_key()
    }

    #[test]
    fn address_string_round_trips_under_prefix() {
        let address = Address::from_public_key(&sample_key());
        let text = address.to_prefixed_string("BTS");
        assert!(text.starts_with("BTS"));
        assert_eq!(Address::from_string(&text, Some("BTS")).unwrap(), address);
    }

    #[test]
    fn round_trips_without_an_explicit_prefix() {
        let address = Address::from_public_key(&sample_key());
        let text = address.to_prefixed_string("GPH");
        assert_eq!(Address::from_string(&text, None).unwrap(), address);
    }

    #[test]
    fn rejects_a_wrong_prefix() {
        let text = Address::from_public_key(&sample_key()).to_prefixed_string("BTS");
        assert!(Address::from_string(&text, Some("GPH")).is_err());
    }

    #[test]
    fn rejects_a_corrupted_checksum() {
        let text = Address::from_public_key(&sample_key()).to_prefixed_string("BTS");
        let mut corrupted = text.into_bytes();
        let last = corrupted.len() - 1;
        corrupted[last] = if corrupted[last] == b'A' { b'B' } else { b'A' };
        let corrupted = String::from_utf8(corrupted).unwrap();
        assert!(Address::from_string(&corrupted, Some("BTS")).is_err());
    }

    #[test]
    fn derivation_is_deterministic_and_twenty_bytes() {
        let key = sample_key();
        assert_eq!(
            Address::from_public_key(&key),
            Address::from_public_key(&key)
        );
        assert_eq!(Address::from_public_key(&key).as_bytes().len(), 20);
    }
}
