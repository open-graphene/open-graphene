use graphene::AccountKeys;

// Pure key derivation: no node, no network. Log in to an account with just its name and password,
// re-deriving the same owner/active/memo keys any Graphene wallet would.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keys = AccountKeys::derive("someaccountname", "somereallylongpassword");

    println!(
        "owner:  {}",
        keys.owner.to_public_key().to_prefixed_string("BTS")
    );
    println!(
        "active: {}",
        keys.active.to_public_key().to_prefixed_string("BTS")
    );
    println!(
        "memo:   {}",
        keys.memo.to_public_key().to_prefixed_string("BTS")
    );
    Ok(())
}
