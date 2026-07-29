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

    /// Assemble a signature from the `r`/`s` components an external signer returned.
    ///
    /// Graphene needs the recovery id that plain ECDSA signers omit: pass it when the signer
    /// supplies one, or `None` to try all four and keep whichever recovers `public_key`.
    /// Returns `None` when no recovery id matches — the components were not produced by that
    /// key over this digest.
    ///
    /// Canonicality is deliberately not checked here. A valid but non-canonical signature is a
    /// retry (sign a fresh digest), not malformed input, so callers check
    /// [`is_canonical`](Self::is_canonical) and react in their own vocabulary.
    pub fn from_components(
        r: &[u8; 32],
        s: &[u8; 32],
        recovery_id: Option<i32>,
        digest: [u8; 32],
        public_key: &PublicKey,
    ) -> Option<Self> {
        let all = [0, 1, 2, 3];
        let candidates: &[i32] = match &recovery_id {
            Some(id) => std::slice::from_ref(id),
            None => &all,
        };

        for &id in candidates {
            if !(0..=3).contains(&id) {
                continue;
            }
            let mut compact = [0u8; 65];
            compact[0] = 27 + 4 + id as u8;
            compact[1..33].copy_from_slice(r);
            compact[33..65].copy_from_slice(s);

            // A wrong recovery id makes recovery fail outright rather than return a
            // mismatching key, so an error here means "not this id", not "broken input".
            if public_key.verify(digest, &compact).unwrap_or(false) {
                return Some(Self(compact));
            }
        }

        None
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

    /// Whether the signature satisfies fc's canonicality predicate (no high bit
    /// and no redundant leading zero in either component's first byte —
    /// `fc::ecc::public_key::is_canonical`, not a low-`s` check). Signatures
    /// produced by [`sign`](Self::sign) always are; check imported signatures
    /// before relying on them.
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

    fn components(signature: &Signature) -> ([u8; 32], [u8; 32], i32) {
        let bytes = signature.as_bytes();
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&bytes[1..33]);
        s.copy_from_slice(&bytes[33..65]);
        (r, s, i32::from(bytes[0]) - 27 - 4)
    }

    #[test]
    fn rebuilds_from_components_with_a_given_recovery_id() {
        let key = key();
        let digest = sha256_bytes(b"external signer");
        let signature = Signature::sign(digest, &key).unwrap();
        let (r, s, recid) = components(&signature);

        let rebuilt =
            Signature::from_components(&r, &s, Some(recid), digest, &key.to_public_key()).unwrap();
        assert_eq!(rebuilt, signature);
    }

    #[test]
    fn recovers_the_recovery_id_when_the_signer_omits_it() {
        let key = key();
        let digest = sha256_bytes(b"no recid supplied");
        let signature = Signature::sign(digest, &key).unwrap();
        let (r, s, _) = components(&signature);

        let rebuilt =
            Signature::from_components(&r, &s, None, digest, &key.to_public_key()).unwrap();
        assert_eq!(rebuilt, signature);
    }

    #[test]
    fn returns_none_for_components_from_another_key() {
        let digest = sha256_bytes(b"wrong key");
        let signature = Signature::sign(digest, &key()).unwrap();
        let (r, s, recid) = components(&signature);

        let other = PrivateKey::from_seed(b"a different signer")
            .unwrap()
            .to_public_key();
        assert!(Signature::from_components(&r, &s, Some(recid), digest, &other).is_none());
        assert!(Signature::from_components(&r, &s, None, digest, &other).is_none());
    }

    #[test]
    fn returns_none_for_an_out_of_range_recovery_id() {
        let key = key();
        let digest = sha256_bytes(b"bad recid");
        let signature = Signature::sign(digest, &key).unwrap();
        let (r, s, _) = components(&signature);

        assert!(Signature::from_components(&r, &s, Some(9), digest, &key.to_public_key()).is_none());
    }
}
