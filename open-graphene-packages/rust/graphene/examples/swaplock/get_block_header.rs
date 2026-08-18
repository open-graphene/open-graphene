use graphene::Graphene;

// Read just a block's header (no transactions). We take the current height from the dynamic global
// properties, pull its header and print witness + timestamp, then show a future height is None.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

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

    let head = swaplock
        .database()
        .get_dynamic_global_properties()
        .await?
        .head_block_number as u32;

    match swaplock.database().block_header(head).get().await? {
        Some(header) => {
            // Typed MaybeSignedBlockHeader: real fields, no JSON indexing.
            let signed = if header.witness_signature.is_some() {
                "signed"
            } else {
                "unsigned"
            };
            println!(
                "block {head} header: witness {} at {} ({signed})",
                header.witness.0, header.timestamp
            );
        }
        None => println!("block {head} header: not produced yet"),
    }

    let future = head + 1_000_000;
    match swaplock.database().get_block_header(future).await? {
        Some(_) => println!("block {future} header: produced (unexpected)"),
        None => println!("block {future} header: not produced yet"),
    }
    Ok(())
}
