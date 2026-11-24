#[path = "common.rs"]
mod common;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mirrornet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .mirrornet()
        .connect()
        .await?;

    let mut subscription = mirrornet
        .database()
        .asset_by_symbol("MBT")
        .subscribe()
        .await?;
    println!(
        "subscribed to asset: {} {} precision {}",
        subscription.initial().id.0,
        subscription.initial().symbol,
        subscription.initial().precision
    );
    println!("listening for asset updates; press Ctrl-C to stop");

    let mut updates = 0_u64;

    loop {
        let update = subscription.next_update().await?;
        updates += 1;
        println!(
            "update #{updates}: {} {} precision {}",
            update.id.0, update.symbol, update.precision
        );
    }
}
