//! Brain keys: deterministic key derivation from a memorable passphrase.
//!
//! A brain key is a string of words a person can write down and later recover their whole account
//! from. Derivation matches `bitsharesjs` exactly, so the same words produce the same keys in any
//! Graphene wallet. Generating a fresh random brain key (the dictionary picker) is not here yet.

use sha2::{Digest, Sha256, Sha512};

use crate::PrivateKey;

/// A normalised brain key. Treat the words like a master password: anyone with them owns the account.
#[derive(Clone, PartialEq, Eq)]
pub struct BrainKey(String);

impl BrainKey {
    /// Take a raw passphrase and normalise it: trim, then collapse every run of whitespace to a
    /// single space (matching bitsharesjs), so casual spacing differences still derive the same keys.
    pub fn new(passphrase: &str) -> Self {
        Self(normalize(passphrase))
    }

    /// The normalised words, e.g. to show the owner for safekeeping.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Derive the private key at `sequence` (use 0 for the account's first key, higher numbers for
    /// additional keys from the same brain key). Matches bitsharesjs: `sha256(sha512(words + " " + sequence))`.
    pub fn private_key(&self, sequence: u32) -> PrivateKey {
        let seed = format!("{} {sequence}", self.0);
        let stretched = Sha512::digest(seed.as_bytes());
        let secret: [u8; 32] = Sha256::digest(stretched).into();
        PrivateKey::from_bytes(secret).expect("a sha256 digest is a valid secp256k1 scalar")
    }
}

impl std::fmt::Debug for BrainKey {
    /// Redacts the words so the brain key can't leak into logs or panics.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrainKey(***)")
    }
}

/// Trim and collapse runs of ASCII whitespace (`\t \n \v \f \r` and space) to single spaces, the
/// same set bitsharesjs normalises on, so derivation is stable across formatting.
fn normalize(passphrase: &str) -> String {
    passphrase
        .split([' ', '\t', '\n', '\u{0b}', '\u{0c}', '\r'])
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORDS: &str = "BRAINKEY EXAMPLE WORDS FOR A WALLET ACCOUNT SECRET PHRASE";

    #[test]
    fn same_words_derive_the_same_key() {
        let a = BrainKey::new(WORDS).private_key(0);
        let b = BrainKey::new(WORDS).private_key(0);
        assert_eq!(a.to_wif(), b.to_wif());
    }

    #[test]
    fn spacing_is_normalised_so_it_does_not_change_the_key() {
        let tidy = BrainKey::new("the quick brown fox").private_key(0);
        let messy = BrainKey::new("  the\tquick\n\n brown   fox  ").private_key(0);
        assert_eq!(tidy.to_wif(), messy.to_wif());
        assert_eq!(
            BrainKey::new("  the\tquick\n\n brown   fox  ").as_str(),
            "the quick brown fox"
        );
    }

    #[test]
    fn sequence_selects_different_keys() {
        let bk = BrainKey::new(WORDS);
        assert_ne!(bk.private_key(0).to_wif(), bk.private_key(1).to_wif());
    }

    #[test]
    fn debug_does_not_leak_the_words() {
        assert_eq!(format!("{:?}", BrainKey::new(WORDS)), "BrainKey(***)");
    }
}
