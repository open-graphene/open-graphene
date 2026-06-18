//! A typed compact ECDSA [`Signature`]: the 65-byte recoverable form Graphene puts on the wire.
//!
//! Ergonomic wrapper over the byte-level signing primitives, so callers hold a signature value with
//! `sign`/`verify`/`recover` methods instead of passing loose `[u8; 65]` around.

use crate::{
    FcSerializeError, PrivateKey, PublicKey, Result, is_graphene_canonical_compact_signature,
};

/// A 65-byte canonical compact secp256k1 signature (recovery id + r + s).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature([u8; 65]);

impl Signature {
    /// Sign a 32-byte `digest` with `private_key`, producing a canonical compact signature.
    pub fn sign(digest: [u8; 32], private_key: &PrivateKey) -> Result<Self> {
        Ok(Self(private_key.sign(digest)?))
    }

    /// Wrap 65 raw signature bytes.
    pub fn from_bytes(bytes: [u8; 65]) -> Self {
        Self(bytes)
    }

    /// Parse a hex-encoded signature (the on-wire form).
    pub fn from_hex(value: &str) -> Result<Self> {
        let bytes = hex::decode(value.trim()).map_err(|_| FcSerializeError::InvalidFixedBytes {
            type_name: "Signature",
            expected_len: 65,
            actual_len: 0,
        })?;
        let bytes: [u8; 65] =
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| FcSerializeError::InvalidFixedBytes {
                    type_name: "Signature",
                    expected_len: 65,
                    actual_len: bytes.len(),
                })?;
        Ok(Self(bytes))
    }

    /// Lowercase hex encoding (the on-wire form).
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Recover the signer's public key straight from the signature over `digest`.
    pub fn recover(&self, digest: [u8; 32]) -> Result<PublicKey> {
        PublicKey::recover(digest, &self.0)
    }

    /// Whether this signature over `digest` was produced by `public_key`'s private key.
    pub fn verify(&self, digest: [u8; 32], public_key: &PublicKey) -> Result<bool> {
        public_key.verify(digest, &self.0)
    }

    /// Whether the signature is in Graphene's canonical low-`s` form (it always is when produced by
    /// [`sign`](Self::sign); check imported signatures before relying on them).
    pub fn is_canonical(&self) -> bool {
        is_graphene_canonical_compact_signature(&self.0)
    }

    /// The raw 65 signature bytes.
    pub fn as_bytes(&self) -> &[u8; 65] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha256_bytes;

    fn key() -> PrivateKey {
        PrivateKey::from_seed(b"open-graphene signature example").unwrap()
    }

    #[test]
    fn sign_verifies_against_the_public_key() {
        let key = key();
        let digest = sha256_bytes(b"sign me");
        let signature = Signature::sign(digest, &key).unwrap();
        assert!(signature.verify(digest, &key.to_public_key()).unwrap());
    }

    #[test]
    fn recovers_the_signer() {
        let key = key();
        let digest = sha256_bytes(b"recover me");
        let signature = Signature::sign(digest, &key).unwrap();
        assert_eq!(signature.recover(digest).unwrap(), key.to_public_key());
    }

    #[test]
    fn hex_round_trips_and_is_canonical() {
        let digest = sha256_bytes(b"hex me");
        let signature = Signature::sign(digest, &key()).unwrap();
        assert!(signature.is_canonical());
        assert_eq!(Signature::from_hex(&signature.to_hex()).unwrap(), signature);
    }

    #[test]
    fn rejects_wrong_length_hex() {
        assert!(Signature::from_hex("abcd").is_err());
    }
}
