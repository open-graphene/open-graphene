use open_graphene_fc::{PrivateKey, encrypt_with_checksum};
fn main() {
    let alice = PrivateKey::from_seed(b"alice").unwrap();
    let bob = PrivateKey::from_seed(b"bob").unwrap();
    let nonce = 123456789;
    println!(
        "{}",
        serde_json::json!({
            "alicePrivateKeyHex":hex::encode(alice.as_bytes()),
            "alicePublicKeyHex":hex::encode(alice.to_public_key().as_bytes()),
            "bobPrivateKeyHex":hex::encode(bob.as_bytes()),
            "bobPublicKeyHex":hex::encode(bob.to_public_key().as_bytes()),
            "nonce":nonce.to_string(),
            "message":"hello bob",
            "ciphertextHex":hex::encode(encrypt_with_checksum(&alice,&bob.to_public_key(),nonce,b"hello bob"))
        })
    );
}
