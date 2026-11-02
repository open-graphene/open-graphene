#[path = "common.rs"]
mod common;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let properties = actanet.database().chain_properties().get().await?;

    println!("id: {}", properties.id.0);
    println!("chain id: {}", properties.chain_id);
    println!(
        "immutable parameters: {:?}",
        properties.immutable_parameters
    );

    Ok(())
}
