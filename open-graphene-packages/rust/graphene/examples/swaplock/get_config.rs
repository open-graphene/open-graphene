use graphene::Graphene;

// Read the chain's fixed config constants. Plain read, no keys. We print a couple of well-known
// GRAPHENE_* parameters to show the map came back.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

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

    let config = swaplock.database().config().get().await?;

    let symbol = config["GRAPHENE_SYMBOL"].as_str().unwrap_or("?");
    let keys = config.len();
    println!("chain symbol: {symbol}");
    println!("config carries {keys} constants");
    if let Some(max_block) = config.get("GRAPHENE_SOFT_MAX_BLOCK_SIZE") {
        println!("soft max block size: {max_block}");
    }
    Ok(())
}
