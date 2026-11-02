#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey};

// Node-verify the account lifecycle builders without committing anything. account_create,
// account_upgrade, account_whitelist and account_transfer are irreversible or privileged, so we
// only prepare() them: that round-trips get_required_fees, so the node parses, validates and prices
// each operation. We deliberately do not sign or broadcast them.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let account = std::env::var("ACTANET_ACCOUNT").unwrap_or_else(|_| "actanet".to_string());
    let throwaway_key = PrivateKey::from_seed(b"open-graphene-new-account")?
        .to_public_key()
        .to_prefixed_string(graphene::PUBLIC_KEY_PREFIX);

    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let id = actanet
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    actanet
        .operations()
        .account_create(&id, "og-example-acct-123")
        .keys(&throwaway_key)
        .prepare()
        .await?;
    println!("account_create: node-priced");

    actanet.operations().account_upgrade(&id).prepare().await?;
    println!("account_upgrade: node-priced");

    actanet
        .operations()
        .account_whitelist(&id, "1.2.0")
        .white_listed()
        .prepare()
        .await?;
    println!("account_whitelist: node-priced");

    actanet
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
