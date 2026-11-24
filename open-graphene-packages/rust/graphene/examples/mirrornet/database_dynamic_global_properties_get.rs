#[path = "common.rs"]
mod common;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mirrornet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .mirrornet()
        .connect()
        .await?;

    let properties = mirrornet
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
