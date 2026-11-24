#[path = "common.rs"]
mod common;

use graphene::{Graphene, ReconnectPolicy};

// Show the reconnect machinery against the node: read the head block, force a reconnect (re-dial
// the same node, rediscover api ids, verify the chain id is unchanged), then read again to prove
// the session still works. Read calls also reconnect themselves on a dropped connection per the
// policy; here we trigger it by hand.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let mut mirrornet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .mirrornet()
        .connect()
        .await?;

    // Read calls retry on a dropped connection per this policy (default: 3 tries with backoff).
    mirrornet.set_reconnect_policy(ReconnectPolicy::default());

    let before = mirrornet
        .database()
        .get_dynamic_global_properties()
        .await?
        .head_block_number;
    println!("head before reconnect: {before}");

    mirrornet.reconnect().await?;
    println!("reconnected");

    let after = mirrornet
        .database()
        .get_dynamic_global_properties()
        .await?
        .head_block_number;
    println!(
        "head after reconnect:  {after} (session still works: {})",
        after >= before
    );
    Ok(())
}
