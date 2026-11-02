#[path = "common.rs"]
mod common;

use graphene::Graphene;

// Read a block off the chain. We ask the dynamic global properties for the current height, pull
// that block and print who produced it and how many transactions it sealed, then show that a
// height the chain has not reached yet comes back as None.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let head = actanet
        .database()
        .get_dynamic_global_properties()
        .await?
        .head_block_number as u32;

    match actanet.database().block(head).get().await? {
        Some(block) => {
            // Typed SignedBlock: fields are real, no JSON indexing. The witness signature parses
            // straight from the node's hex into bytes.
            println!(
                "block {head}: witness {}, sealed {} transaction(s) at {}, signature {} bytes",
                block.witness.0,
                block.transactions.len(),
                block.timestamp,
                block.witness_signature.0.len()
            );
        }
        None => println!("block {head}: not produced yet"),
    }

    let future = head + 1_000_000;
    match actanet.database().get_block(future).await? {
        Some(_) => println!("block {future}: produced (unexpected)"),
        None => println!("block {future}: not produced yet"),
    }
    Ok(())
}
