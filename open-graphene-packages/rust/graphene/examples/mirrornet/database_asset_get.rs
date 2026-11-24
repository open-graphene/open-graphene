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

    let asset = mirrornet.database().asset_by_symbol("MBT").get().await?;
    println!(
        "asset by symbol: {} {} precision {}",
        asset.id.0, asset.symbol, asset.precision
    );

    let same_asset = mirrornet.database().asset_by_id(&asset.id.0).get().await?;
    println!(
        "asset by id: {} {} precision {}",
        same_asset.id.0, same_asset.symbol, same_asset.precision
    );

    Ok(())
}
