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
        .chain_id("1e9aea9e936607ca2cce053765c39c995c591ef23d85332ec63d6f06a0704f86")
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
