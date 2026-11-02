use graphene::{PrivateKey, decrypt_with_checksum, encrypt_with_checksum};

// Pure memo crypto: no node, no network. Encrypt a memo to a recipient, then have them read it.
// The encrypted bytes are what goes into a transaction's `encrypted_memo` field.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sender = PrivateKey::from_seed(b"sender wallet seed")?;
    let recipient = PrivateKey::from_seed(b"recipient wallet seed")?;

    // Store this nonce next to the memo; the recipient needs it to decrypt.
    let nonce = 289_662_526_069_530_675;
    let cipher = encrypt_with_checksum(&sender, &recipient.to_public_key(), nonce, b"lunch money");
    println!("encrypted_memo: {} bytes", cipher.len());

    // Recipient decrypts with their private key and the sender's public key.
    let plain = decrypt_with_checksum(&recipient, &sender.to_public_key(), nonce, &cipher)?;
    println!("decrypted: {}", String::from_utf8_lossy(&plain));
    Ok(())
}
