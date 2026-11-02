#[path = "common.rs"]
mod common;

use std::time::Duration;

use graphene::Graphene;

// Show the reactive cache: subscribe a ChainStore to the dynamic global properties (2.1.0), then
// read the cached head block twice a few seconds apart. The value advances on its own, pushed by
// the node's subscription, without us calling get_objects again.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let live = actanet.into_live()?;
    let store = live.database().chain_store(["2.1.0"]).await?;

    let head = |label: &str| {
        let value = store.get("2.1.0");
        let number = value
            .as_ref()
            .and_then(|object| object["head_block_number"].as_u64())
            .unwrap_or(0);
        println!("{label}: head_block_number = {number}");
        number
    };

    let first = head("seeded");
    tokio::time::sleep(Duration::from_secs(7)).await;
    let second = head("after 7s");

    println!(
        "store has {} object(s); advanced on its own: {}",
        store.len(),
        second > first
    );
    Ok(())
}
