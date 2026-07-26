use graphene::Graphene;

// Two read-only RPC queries: list the chain's assets and look up account names. Both paginate from
// a lower bound, so this is also how you would drive asset discovery or account autocomplete.
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

    println!("assets:");
    for asset in swaplock.database().list_assets("").limit(10).get().await? {
        println!(
            "  {} {} precision {}",
            asset.id.0, asset.symbol, asset.precision
        );
    }

    println!("accounts:");
    for (name, id) in swaplock
        .database()
        .lookup_accounts("")
        .limit(10)
        .get()
        .await?
    {
        println!("  {id} {name}");
    }
    Ok(())
}
