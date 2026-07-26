use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut subscription = swaplock
        .database()
        .asset_by_symbol("BTS")
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
