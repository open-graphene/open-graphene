use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9")
        .prefix("BTS")
        .lowest_latency()
        .build()?
        .swaplock();

    // Health check: which node answers fastest right now?
    println!("latency report (fastest first):");
    for entry in builder.probe_latencies().await? {
        println!("  {:>6} ms  {}", entry.latency.as_millis(), entry.server);
    }

    // Connect using the lowest-latency strategy, then make a call on the winner.
    let mut swaplock = builder.connect().await?;
    let chain_id = swaplock.database().chain_id().get().await?;

    println!("connected to fastest node; chain id: {chain_id}");
    Ok(())
}
