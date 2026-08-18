use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
