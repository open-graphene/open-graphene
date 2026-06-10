//! Memo encryption: the AES-with-checksum scheme Graphene uses for `encrypted_memo`.
//!
//! A faithful port of `bitsharesjs` `Aes.encrypt_with_checksum` / `decrypt_with_checksum`, so a
//! memo produced here decrypts in any Graphene client and vice versa. Pair these with
//! [`PrivateKey::get_shared_secret`](crate::PrivateKey::get_shared_secret).

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use sha2::{Digest, Sha256, Sha512};

use crate::{FcSerializeError, PrivateKey, PublicKey, Result};

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Encrypt `message` so only the holder of the matching keys can read it.
///
/// `from` is the sender's private key, `to` the recipient's public key, `nonce` a number that must
/// be stored alongside the result (it keys the cipher). The returned bytes go into `encrypted_memo`.
/// Decrypt with [`decrypt_with_checksum`] using the mirror key pair.
pub fn encrypt_with_checksum(
    from: &PrivateKey,
    to: &PublicKey,
    nonce: u64,
    message: &[u8],
) -> Vec<u8> {
    let (key, iv) = derive_key_iv(from.get_shared_secret(to), nonce);

    let checksum = Sha256::digest(message);
    let mut payload = Vec::with_capacity(4 + message.len());
    payload.extend_from_slice(&checksum[..4]);
    payload.extend_from_slice(message);

    Aes256CbcEnc::new_from_slices(&key, &iv)
        .expect("32-byte key and 16-byte iv")
        .encrypt_padded_vec_mut::<Pkcs7>(&payload)
}

/// Decrypt a memo produced by [`encrypt_with_checksum`], returning the original message.
///
/// `secret` is the reader's private key and `peer` the other party's public key; `nonce` must be
/// the one used to encrypt. Fails if the keys or nonce are wrong (the embedded checksum won't match).
pub fn decrypt_with_checksum(
    secret: &PrivateKey,
    peer: &PublicKey,
    nonce: u64,
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    let (key, iv) = derive_key_iv(secret.get_shared_secret(peer), nonce);

    let plaintext = Aes256CbcDec::new_from_slices(&key, &iv)
        .expect("32-byte key and 16-byte iv")
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|_| FcSerializeError::MemoDecryptFailed {
            reason: "AES decrypt or padding failed (wrong key or nonce?)",
        })?;

    if plaintext.len() < 4 {
        return Err(FcSerializeError::MemoDecryptFailed {
            reason: "decrypted payload is shorter than its checksum",
        });
    }

    let (checksum, message) = plaintext.split_at(4);
    if checksum != &Sha256::digest(message)[..4] {
        return Err(FcSerializeError::MemoDecryptFailed {
            reason: "checksum mismatch (wrong key or nonce?)",
        });
    }
    Ok(message.to_vec())
}

/// The AES key (32 bytes) and IV (16 bytes): `sha512(nonce_decimal ++ hex(shared_secret))`.
fn derive_key_iv(shared_secret: [u8; 64], nonce: u64) -> ([u8; 32], [u8; 16]) {
    let mut seed = nonce.to_string().into_bytes();
    for &byte in shared_secret.iter() {
        seed.push(HEX[(byte >> 4) as usize]);
        seed.push(HEX[(byte & 0x0f) as usize]);
    }
    let hash = Sha512::digest(&seed);

    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    key.copy_from_slice(&hash[..32]);
    iv.copy_from_slice(&hash[32..48]);
    (key, iv)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice() -> PrivateKey {
        PrivateKey::from_seed(b"alice").unwrap()
    }
    fn bob() -> PrivateKey {
        PrivateKey::from_seed(b"bob").unwrap()
    }

    #[test]
    fn recipient_decrypts_what_the_sender_encrypted() {
        let nonce = 123_456_789;
        let cipher = encrypt_with_checksum(&alice(), &bob().to_public_key(), nonce, b"hello bob");
        // Bob reads it with his private key and Alice's public key (shared secret is symmetric).
        let plain =
            decrypt_with_checksum(&bob(), &alice().to_public_key(), nonce, &cipher).unwrap();
        assert_eq!(plain, b"hello bob");
    }

    #[test]
    fn empty_message_round_trips() {
        let cipher = encrypt_with_checksum(&alice(), &bob().to_public_key(), 1, b"");
        assert_eq!(
            decrypt_with_checksum(&bob(), &alice().to_public_key(), 1, &cipher).unwrap(),
            b""
        );
    }

    #[test]
    fn wrong_nonce_fails_the_checksum() {
        let cipher = encrypt_with_checksum(&alice(), &bob().to_public_key(), 7, b"secret");
        assert!(decrypt_with_checksum(&bob(), &alice().to_public_key(), 8, &cipher).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let mallory = PrivateKey::from_seed(b"mallory").unwrap();
        let cipher = encrypt_with_checksum(&alice(), &bob().to_public_key(), 7, b"secret");
        assert!(decrypt_with_checksum(&mallory, &alice().to_public_key(), 7, &cipher).is_err());
    }
}
