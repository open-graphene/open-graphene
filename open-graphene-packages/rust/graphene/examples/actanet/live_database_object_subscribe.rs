#[path = "common.rs"]
mod common;

use std::time::Duration;

use graphene::Graphene;

const WAIT_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let live = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect_live()
        .await?;

    let account = live
        .database()
        .account_by_id("1.2.100")
        .subscribe_timeout(WAIT_TIMEOUT)
        .await?;
    println!(
        "live account subscription initial: {} {}",
        account.initial().id.0,
        account.initial().name
    );

    let asset = live
        .database()
        .asset_by_id("1.3.0")
        .subscribe_timeout(WAIT_TIMEOUT)
        .await?;
    println!(
        "live asset subscription initial: {} {} precision {}",
        asset.initial().id.0,
        asset.initial().symbol,
        asset.initial().precision
    );

    println!("registered typed live account and asset subscriptions on one websocket");

    Ok(())
}
