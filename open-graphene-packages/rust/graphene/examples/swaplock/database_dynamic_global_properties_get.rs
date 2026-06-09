use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
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

    let properties = swaplock
        .database()
        .dynamic_global_properties()
        .get()
        .await?;

    println!("id: {}", properties.id.0);
    println!("head block: {}", properties.head_block_number);
    println!("time: {}", properties.time);
    println!("current witness: {}", properties.current_witness.0);
    println!(
        "last irreversible block: {}",
        properties.last_irreversible_block_num
    );

    Ok(())
}
