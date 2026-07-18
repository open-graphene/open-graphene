use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
