use graphene::Graphene;

async fn connect_to_swaplock() -> Result<(), Box<dyn std::error::Error>> {
    let swaplock = Graphene::swaplock()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
        .prefix("BTS")
        .connect()
        .await?;

    println!("configured servers: {}", swaplock.config().servers().len());
    Ok(())
}

#[allow(dead_code)]
async fn connect_via_generic_builder() -> Result<(), Box<dyn std::error::Error>> {
    let swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    println!("configured prefix: {:?}", swaplock.config().prefix());
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    connect_to_swaplock().await?;
    connect_via_generic_builder().await?;
    Ok(())
}
