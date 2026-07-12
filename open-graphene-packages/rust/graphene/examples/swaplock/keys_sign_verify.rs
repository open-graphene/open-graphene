use graphene::{PrivateKey, PublicKey};

// Pure key handling: no node, no network. Derive a key, round-trip it through WIF and the
// public-key string, sign a digest, then verify and recover the signer.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let private = PrivateKey::from_seed(b"open-graphene demo seed")?;
    let public = private.to_public_key();

    println!("wif:        {}", private.to_wif());
    println!("public key: {}", public.to_prefixed_string("BTS"));

    // A digest is whatever 32 bytes you want signed (here, a hash stand-in).
    let digest = [7u8; 32];
    let signature = private.sign(digest)?;

    println!("verifies:   {}", public.verify(digest, &signature)?);
    println!(
        "recovered == signer: {}",
        PublicKey::recover(digest, &signature)? == public
    );
    Ok(())
}
