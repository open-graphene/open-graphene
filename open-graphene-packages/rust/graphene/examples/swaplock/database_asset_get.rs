use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("e80d8f63b598759059ca8f8627a6c9252bf6ae13ed404e1afbf4ae51b1781837")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let asset = swaplock.database().asset_by_symbol("BTS").get().await?;
    println!(
        "asset by symbol: {} {} precision {}",
        asset.id.0, asset.symbol, asset.precision
    );

    let same_asset = swaplock.database().asset_by_id(&asset.id.0).get().await?;
    println!(
        "asset by id: {} {} precision {}",
        same_asset.id.0, same_asset.symbol, same_asset.precision
    );

    Ok(())
}
