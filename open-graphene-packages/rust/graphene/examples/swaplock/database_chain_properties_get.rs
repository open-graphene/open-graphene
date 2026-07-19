use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("f990ce83af5cf2d55c180ca4bd4b34161ccff2b2f8cc7d4987eea9555153930e")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let properties = swaplock.database().chain_properties().get().await?;

    println!("id: {}", properties.id.0);
    println!("chain id: {}", properties.chain_id);
    println!(
        "immutable parameters: {:?}",
        properties.immutable_parameters
    );

    Ok(())
}
