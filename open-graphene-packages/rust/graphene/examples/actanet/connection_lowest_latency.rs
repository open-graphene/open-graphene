#[path = "common.rs"]
mod common;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .lowest_latency()
        .build()?
        .actanet();

    // Health check: which node answers fastest right now?
    println!("latency report (fastest first):");
    for entry in builder.probe_latencies().await? {
        println!("  {:>6} ms  {}", entry.latency.as_millis(), entry.server);
    }

    // Connect using the lowest-latency strategy, then make a call on the winner.
    let mut actanet = builder.connect().await?;
    let chain_id = actanet.database().chain_id().get().await?;

    println!("connected to fastest node; chain id: {chain_id}");
    Ok(())
}
