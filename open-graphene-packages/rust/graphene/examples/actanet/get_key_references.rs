#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey};

// Ask the chain which accounts reference our active public key. We derive the key from the WIF,
// look up our own account id, then check it shows up in the key references. No mutation, plain read.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("ACTANET_ACTIVE_WIF")?;
    let account = std::env::var("ACTANET_ACCOUNT").unwrap_or_else(|_| "actanet".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string(graphene::PUBLIC_KEY_PREFIX);

    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let account_id = actanet
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    let references = actanet
        .database()
        .key_references([public_key.as_str()])
        .get()
        .await?;
    let accounts = references.first().cloned().unwrap_or_default();
    println!("active key {public_key} is referenced by: {accounts:?}");

    if accounts.iter().any(|account| account.0 == account_id) {
        println!("our account {account_id} is in there, as expected");
    } else {
        println!("our account {account_id} was not in the references");
    }
    Ok(())
}
