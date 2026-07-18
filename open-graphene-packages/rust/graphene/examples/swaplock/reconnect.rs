use graphene::{Graphene, ReconnectPolicy};

// Show the reconnect machinery against the node: read the head block, force a reconnect (re-dial
// the same node, rediscover api ids, verify the chain id is unchanged), then read again to prove
// the session still works. Read calls also reconnect themselves on a dropped connection per the
// policy; here we trigger it by hand.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("f2491c85896bb49f936152d59b850ca05bb8e09d9027eb630267697ee483b05e")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    // Read calls retry on a dropped connection per this policy (default: 3 tries with backoff).
    swaplock.set_reconnect_policy(ReconnectPolicy::default());

    let before = swaplock
        .database()
        .get_dynamic_global_properties()
        .await?
        .head_block_number;
    println!("head before reconnect: {before}");

    swaplock.reconnect().await?;
    println!("reconnected");

    let after = swaplock
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
