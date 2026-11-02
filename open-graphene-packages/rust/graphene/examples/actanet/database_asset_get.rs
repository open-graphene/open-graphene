#[path = "common.rs"]
mod common;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let asset = actanet.database().asset_by_symbol("ACTA").get().await?;
    println!(
        "asset by symbol: {} {} precision {}",
        asset.id.0, asset.symbol, asset.precision
    );

    let same_asset = actanet.database().asset_by_id(&asset.id.0).get().await?;
    println!(
        "asset by id: {} {} precision {}",
        same_asset.id.0, same_asset.symbol, same_asset.precision
    );

    Ok(())
}
