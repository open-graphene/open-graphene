use graphene::{Graphene, PrivateKey};

// Ask the chain which accounts reference our active public key. We derive the key from the WIF,
// look up our own account id, then check it shows up in the key references. No mutation, plain read.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("SWAPLOCK_ACTIVE_WIF")?;
    let account = std::env::var("SWAPLOCK_ACCOUNT").unwrap_or_else(|_| "swaplock".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string("BTS");

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let account_id = swaplock
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    let references = swaplock
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
