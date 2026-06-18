//! Brain keys: deterministic key derivation from a memorable passphrase.
//!
//! A brain key is a string of words a person can write down and later recover their whole account
//! from. Derivation matches `bitsharesjs` exactly, so the same words produce the same keys in any
//! Graphene wallet. [`BrainKey::suggest`] generates a fresh random one from a supplied dictionary.

use sha2::{Digest, Sha256, Sha512};

use crate::{FcSerializeError, PrivateKey, Result};

/// How many words a suggested brain key has, matching bitsharesjs.
pub const SUGGESTED_BRAIN_KEY_WORDS: usize = 16;

/// A normalised brain key. Treat the words like a master password: anyone with them owns the account.
#[derive(Clone, PartialEq, Eq)]
pub struct BrainKey(String);

impl BrainKey {
    /// Take a raw passphrase and normalise it: trim, then collapse every run of whitespace to a
    /// single space (matching bitsharesjs), so casual spacing differences still derive the same keys.
    pub fn new(passphrase: &str) -> Self {
        Self(normalize_brain_key(passphrase))
    }

    /// Generate a fresh random brain key by drawing [`SUGGESTED_BRAIN_KEY_WORDS`] words from
    /// `dictionary`, using the OS cryptographic RNG. The bitsharesjs port: pass the same word list
    /// the JS wallets ship (a comma- or whitespace-separated string); each word is chosen uniformly
    /// and independently, so security is `word_count * log2(unique_words)` bits.
    pub fn suggest(dictionary: &str) -> Result<Self> {
        Self::suggest_words(dictionary, SUGGESTED_BRAIN_KEY_WORDS)
    }

    /// Like [`suggest`](Self::suggest) but with a caller-chosen `word_count`.
    pub fn suggest_words(dictionary: &str, word_count: usize) -> Result<Self> {
        let words: Vec<&str> = dictionary
            .split([',', ' ', '\t', '\n', '\u{0b}', '\u{0c}', '\r'])
            .filter(|piece| !piece.is_empty())
            .collect();
        if words.is_empty() {
            return Err(FcSerializeError::UnsupportedValue {
                type_name: "BrainKey",
                reason: "dictionary is empty",
            });
        }
        let chosen = (0..word_count)
            .map(|_| Ok(words[random_index(words.len())?]))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self::new(&chosen.join(" ")))
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

/// A uniform random index in `0..len` from the OS CSPRNG, using rejection sampling so the result is
/// unbiased even when `len` does not divide `2^64`.
fn random_index(len: usize) -> Result<usize> {
    let len = len as u64;
    let limit = u64::MAX - (u64::MAX % len);
    loop {
        let mut bytes = [0u8; 8];
        getrandom::getrandom(&mut bytes).map_err(|_| FcSerializeError::UnsupportedValue {
            type_name: "BrainKey",
            reason: "the OS random number generator is unavailable",
        })?;
        let value = u64::from_le_bytes(bytes);
        if value < limit {
            return Ok((value % len) as usize);
        }
    }
}

/// Trim and collapse runs of ASCII whitespace (`\t \n \v \f \r` and space) to single spaces, the
/// same set bitsharesjs `normalize_brainKey` uses. Shared with account-login key derivation so both
/// stay byte-identical to JS.
pub(crate) fn normalize_brain_key(passphrase: &str) -> String {
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

    const DICTIONARY: &str =
        "alpha,bravo,charlie,delta,echo,foxtrot,golf,hotel,india,juliet,kilo,lima";

    #[test]
    fn suggest_draws_the_right_number_of_words_from_the_dictionary() {
        let allowed: Vec<&str> = DICTIONARY.split(',').collect();
        let brain = BrainKey::suggest(&DICTIONARY.replace(',', " ")).unwrap();
        let words: Vec<&str> = brain.as_str().split(' ').collect();
        assert_eq!(words.len(), SUGGESTED_BRAIN_KEY_WORDS);
        assert!(words.iter().all(|word| allowed.contains(word)));
    }

    #[test]
    fn suggest_words_honours_the_requested_count() {
        let brain = BrainKey::suggest_words(DICTIONARY, 4).unwrap();
        assert_eq!(brain.as_str().split(' ').count(), 4);
    }

    #[test]
    fn two_suggestions_differ() {
        // With 16 draws from a 12-word list the odds of an exact match are vanishing.
        let a = BrainKey::suggest(DICTIONARY).unwrap();
        let b = BrainKey::suggest(DICTIONARY).unwrap();
        assert_ne!(a.as_str(), b.as_str());
    }

    #[test]
    fn suggest_rejects_an_empty_dictionary() {
        assert!(BrainKey::suggest("   ").is_err());
    }

    #[test]
    fn a_suggested_key_derives_a_usable_private_key() {
        let brain = BrainKey::suggest(DICTIONARY).unwrap();
        assert!(!brain.private_key(0).to_wif().is_empty());
    }
}
