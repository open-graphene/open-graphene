use graphene::BrainKey;

// Pure key derivation: no node, no network. Recover account keys from a brain key the same way
// any Graphene wallet does, so the same words always give the same keys.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let brain_key = BrainKey::new("  brainkey  example words for a wallet account secret phrase  ");
    println!("normalised: {}", brain_key.as_str());

    // sequence 0 is the first key; bump it for extra keys from the same brain key.
    let owner = brain_key.private_key(0);
    let active = brain_key.private_key(1);

    println!(
        "owner public key:  {}",
        owner.to_public_key().to_prefixed_string("BTS")
    );
    println!(
        "active public key: {}",
        active.to_public_key().to_prefixed_string("BTS")
    );
    Ok(())
}
