use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
