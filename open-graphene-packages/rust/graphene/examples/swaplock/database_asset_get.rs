use graphene::Graphene;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
