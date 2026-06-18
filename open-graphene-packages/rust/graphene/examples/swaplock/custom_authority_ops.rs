use graphene::{Graphene, PrivateKey};

// Exercise the custom-authority family: an account delegates the right to sign a specific operation
// type to another key, then tweaks or revokes it. Creating one needs an authority on the account
// that our test key does not satisfy here, so we node-verify each at prepare() (the node prices and
// validates the op) without broadcasting.
const TRANSFER_OP_TYPE: u32 = 0; // delegate the right to sign transfers

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("SWAPLOCK_ACTIVE_WIF")?;
    let account = std::env::var("SWAPLOCK_ACCOUNT").unwrap_or_else(|_| "swaplock".to_string());
    let pk = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string("BTS");

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let id = swaplock
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    // Delegate transfer-signing to a key (unconstrained); the validity window defaults to one year.
    swaplock
        .operations()
        .custom_authority_create(&id, TRANSFER_OP_TYPE)
        .auth_key(&pk)
        .prepare()
        .await?;

    // Update and delete act on an existing authority id (1.17.0 here).
    swaplock
        .operations()
        .custom_authority_update(&id, "1.17.0")
        .enabled(false)
        .prepare()
        .await?;
    swaplock
        .operations()
        .custom_authority_delete(&id, "1.17.0")
        .prepare()
        .await?;

    println!("custom_authority_create / update / delete: node-priced (not broadcast)");
    Ok(())
}
