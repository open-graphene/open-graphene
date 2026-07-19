use graphene::{Graphene, PrivateKey};

// Node-verify the account lifecycle builders without committing anything. account_create,
// account_upgrade, account_whitelist and account_transfer are irreversible or privileged, so we
// only prepare() them: that round-trips get_required_fees, so the node parses, validates and prices
// each operation. We deliberately do not sign or broadcast them.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let account = std::env::var("SWAPLOCK_ACCOUNT").unwrap_or_else(|_| "swaplock".to_string());
    let throwaway_key = PrivateKey::from_seed(b"open-graphene-new-account")?
        .to_public_key()
        .to_prefixed_string("BTS");

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("f990ce83af5cf2d55c180ca4bd4b34161ccff2b2f8cc7d4987eea9555153930e")
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

    swaplock
        .operations()
        .account_create(&id, "og-example-acct-123")
        .keys(&throwaway_key)
        .prepare()
        .await?;
    println!("account_create: node-priced");

    swaplock.operations().account_upgrade(&id).prepare().await?;
    println!("account_upgrade: node-priced");

    swaplock
        .operations()
        .account_whitelist(&id, "1.2.0")
        .white_listed()
        .prepare()
        .await?;
    println!("account_whitelist: node-priced");

    swaplock
        .operations()
        .account_transfer(&id, "1.2.0")
        .prepare()
        .await?;
    println!(
        "account_transfer: node-priced (not broadcast \u{2014} it would give away the account)"
    );

    println!("all account ops node-verified; none broadcast");
    Ok(())
}
