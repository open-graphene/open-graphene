use graphene::Graphene;

// Exercise override_transfer: the issuer of a user asset forcibly moves it between accounts.
//
// This only applies to an asset whose override_authority permission is enabled, which our example
// asset does not have, so we node-verify at prepare() (the node prices/validates the op) without
// broadcasting.
const ASSET: &str = "1.3.102"; // a user asset we issue
const HOLDER: &str = "1.2.0"; // the account we would claw units back from

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let account = std::env::var("SWAPLOCK_ACCOUNT").unwrap_or_else(|_| "swaplock".to_string());

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("95acb1f01e4afd0dd8b61f99377911a2f0ac6e6420925e01630f1d3580c89758")
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

    // As the issuer, pull 1 unit of our asset from HOLDER back to ourselves.
    swaplock
        .operations()
        .override_transfer(&id, HOLDER, &id)
        .amount(1, ASSET)
        .prepare()
        .await?;
    println!("override_transfer: node-priced (not broadcast)");
    Ok(())
}
