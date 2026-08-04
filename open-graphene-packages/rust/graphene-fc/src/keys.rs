//! Typed ECC keys: [`PrivateKey`] and [`PublicKey`].
//!
//! Ergonomic wrappers over the byte-level primitives in this crate, so callers pass WIF strings,
//! public-key strings and signatures around instead of loose `[u8; 32]` / `[u8; 33]` and a pile
//! of free functions.

use ripemd::{Digest, Ripemd160};
use secp256k1::{PublicKey as Secp256k1PublicKey, Scalar, Secp256k1, SecretKey};
use sha2::Sha512;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{
    FcSerializeError, Result, decode_public_key, decode_wif_private_key,
    recover_public_key_from_compact_signature, sha256_bytes, sign_digest_compact,
    verify_compact_signature_public_key,
};

/// A secp256k1 private key. Keep it secret: treat it like a password, never log or display it.
/// The key bytes are zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct PrivateKey([u8; 32]);

impl PrivateKey {
    /// Parse a Wallet Import Format string (the `5...` keys exported by wallets).
    pub fn from_wif(wif: &str) -> Result<Self> {
        Ok(Self(decode_wif_private_key(wif)?))
    }

    /// Wrap 32 raw key bytes, rejecting scalars outside the curve order.
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self> {
        SecretKey::from_slice(&bytes).map_err(|_| FcSerializeError::InvalidPrivateKey {
            reason: "private key scalar is out of range",
        })?;
        Ok(Self(bytes))
    }

    /// Derive a key deterministically from any seed (passphrase, brain-key words) as `sha256(seed)`.
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        Self::from_bytes(sha256_bytes(seed))
    }

    /// Render back to WIF for storage or display by the wallet owner.
    pub fn to_wif(&self) -> String {
        let mut payload = Vec::with_capacity(37);
        payload.push(0x80);
        payload.extend_from_slice(&self.0);
        let checksum = sha256_bytes(&sha256_bytes(&payload));
        payload.extend_from_slice(&checksum[..4]);
        let wif = bs58::encode(&payload).into_string();
        payload.zeroize();
        wif
    }

    /// The public key that pairs with this private key.
    pub fn to_public_key(&self) -> PublicKey {
        let secret = SecretKey::from_slice(&self.0).expect("validated on construction");
        let public = Secp256k1PublicKey::from_secret_key(&Secp256k1::new(), &secret);
        PublicKey(public.serialize())
    }

    /// Sign a 32-byte digest, producing a canonical 65-byte compact signature.
    pub fn sign(&self, digest: [u8; 32]) -> Result<[u8; 65]> {
        sign_digest_compact(digest, self.0)
    }

    /// Diffie-Hellman shared secret with `other`, the way Graphene keys it: `sha512` of the
    /// 32-byte X coordinate of `self * other`. Symmetric, so both parties derive the same value.
    /// This is the seed behind memo encryption (see [`crate::encrypt_with_checksum`]).
    pub fn get_shared_secret(&self, other: &PublicKey) -> [u8; 64] {
        let scalar = Scalar::from_be_bytes(self.0).expect("private key validated on construction");
        let point = Secp256k1PublicKey::from_slice(&other.0).expect("public key validated");
        let shared = point
            .mul_tweak(&Secp256k1::new(), &scalar)
            .expect("shared point is never the identity for distinct valid keys");
        let mut out = [0u8; 64];
        out.copy_from_slice(&Sha512::digest(&shared.serialize_uncompressed()[1..33]));
        out
    }

    /// The raw 32 key bytes, for callers that need the scalar directly.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for PrivateKey {
    /// Redacts the secret so it can't leak into logs or panics.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PrivateKey(***)")
    }
}

/// A secp256k1 public key in compressed (33-byte) form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey([u8; 33]);

impl PublicKey {
    /// Parse a chain public-key string. Pass the chain `prefix` (e.g. `"BTS"`) to require it,
    /// or `None` to accept whatever prefix the string carries.
    pub fn from_string(value: &str, prefix: Option<&str>) -> Result<Self> {
        Ok(Self(decode_public_key(value, prefix)?))
    }

    /// Wrap 33 raw bytes, rejecting anything that isn't a valid compressed point.
    pub fn from_bytes(bytes: [u8; 33]) -> Result<Self> {
        Secp256k1PublicKey::from_slice(&bytes).map_err(|_| FcSerializeError::InvalidPublicKey {
            value: String::new(),
            expected_prefix: None,
            reason: "not a valid compressed secp256k1 point",
        })?;
        Ok(Self(bytes))
    }

    /// Parse 33 compressed-key bytes given as hex — the shape external signers (HSMs,
    /// custody APIs) hand back. Rejects anything that isn't a compressed point.
    pub fn from_hex(value: &str) -> Result<Self> {
        let decoded =
            hex::decode(value.trim()).map_err(|_| FcSerializeError::InvalidPublicKey {
                value: value.to_string(),
                expected_prefix: None,
                reason: "public key is not valid hex",
            })?;
        let bytes: [u8; 33] =
            decoded
                .try_into()
                .map_err(|_| FcSerializeError::InvalidPublicKey {
                    value: value.to_string(),
                    expected_prefix: None,
                    reason: "expected a 33 byte compressed secp256k1 key",
                })?;
        Self::from_bytes(bytes)
    }

    /// `GRAPHENE_NULL_KEY`: 33 zero bytes. Not a curve point, but legal on chain as the memo
    /// key of accounts that never receive memos — e.g. accounts governed purely by other
    /// accounts. [`PublicKey::from_string`] already accepts it, so this is the constructor
    /// side of the same rule and deliberately skips point validation.
    pub fn null() -> Self {
        Self([0u8; 33])
    }

    /// Render as the chain public-key string (e.g. `BTS6MRy...`) under `prefix`.
    pub fn to_prefixed_string(&self, prefix: &str) -> String {
        let checksum = Ripemd160::digest(self.0);
        let mut payload = Vec::with_capacity(37);
        payload.extend_from_slice(&self.0);
        payload.extend_from_slice(&checksum[..4]);
        format!("{prefix}{}", bs58::encode(payload).into_string())
    }

    /// Whether `signature` over `digest` was produced by the matching private key.
    pub fn verify(&self, digest: [u8; 32], signature: &[u8]) -> Result<bool> {
        verify_compact_signature_public_key(digest, signature, self.0)
    }

    /// Recover the signer's public key straight from a compact signature, no key needed up front.
    pub fn recover(digest: [u8; 32], signature: &[u8]) -> Result<Self> {
        Ok(Self(recover_public_key_from_compact_signature(
            digest, signature,
        )?))
    }

    /// The raw 33 compressed-key bytes.
    pub fn as_bytes(&self) -> &[u8; 33] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_key() -> PrivateKey {
        PrivateKey::from_seed(b"open-graphene example seed").unwrap()
    }

    #[test]
    fn wif_round_trips_through_string() {
        let key = sample_key();
        let restored = PrivateKey::from_wif(&key.to_wif()).unwrap();
        assert_eq!(key.as_bytes(), restored.as_bytes());
    }

    #[test]
    fn public_key_string_round_trips_under_prefix() {
        let public = sample_key().to_public_key();
        let text = public.to_prefixed_string("BTS");
        assert!(text.starts_with("BTS"));
        assert_eq!(PublicKey::from_string(&text, Some("BTS")).unwrap(), public);
    }

    #[test]
    fn sign_verifies_against_derived_public_key() {
        let key = sample_key();
        let public = key.to_public_key();
        let digest = sha256_bytes(b"a message to sign");
        let signature = key.sign(digest).unwrap();
        assert!(public.verify(digest, &signature).unwrap());
    }

    #[test]
    fn recovered_public_key_matches_the_signer() {
        let key = sample_key();
        let digest = sha256_bytes(b"another message");
        let signature = key.sign(digest).unwrap();
        assert_eq!(
            PublicKey::recover(digest, &signature).unwrap(),
            key.to_public_key()
        );
    }

    #[test]
    fn debug_does_not_leak_the_secret() {
        assert_eq!(format!("{:?}", sample_key()), "PrivateKey(***)");
    }

    #[test]
    fn matches_bitsharesjs_known_vector() {
        // bitsharesjs test/ecc/Crypto.js: PrivateKey.fromSeed("1").toPublicKey().toString()
        // under prefix "CBA". Proves from_seed + derive + public-key-string are byte-identical.
        let public = PrivateKey::from_seed(b"1").unwrap().to_public_key();
        assert_eq!(
            public.to_prefixed_string("CBA"),
            "CBA8m5UgaFAAYQRuaNejYdS8FVLVp9Ss3K1qAVk5de6F8s3HnVbvA"
        );
    }

    #[test]
    fn hex_public_key_round_trips_through_the_chain_string() {
        // Compressed secp256k1 generator point.
        let hex_key = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

        let public = PublicKey::from_hex(hex_key).expect("hex decodes");
        let text = public.to_prefixed_string("BTS");

        assert!(text.starts_with("BTS"));
        assert_eq!(PublicKey::from_string(&text, Some("BTS")).unwrap(), public);
        assert_eq!(hex::encode(public.as_bytes()), hex_key);
    }

    #[test]
    fn from_hex_rejects_uncompressed_and_malformed_keys() {
        assert!(PublicKey::from_hex(&format!("04{}", "ab".repeat(64))).is_err());
        assert!(PublicKey::from_hex("zz").is_err());
    }

    #[test]
    fn null_key_matches_the_canonical_graphene_null_key() {
        // 33 zero bytes are not a curve point, but the chain accepts them as
        // GRAPHENE_NULL_KEY — the memo key of accounts that never receive memos.
        assert_eq!(
            PublicKey::null().to_prefixed_string("BTS"),
            "BTS1111111111111111111111111111111114T1Anm"
        );
        assert_eq!(
            PublicKey::from_string("BTS1111111111111111111111111111111114T1Anm", Some("BTS"))
                .unwrap(),
            PublicKey::null()
        );
    }

    #[test]
    fn rejects_a_public_key_with_the_wrong_prefix() {
        let text = sample_key().to_public_key().to_prefixed_string("BTS");
        assert!(PublicKey::from_string(&text, Some("GPH")).is_err());
    }
}
