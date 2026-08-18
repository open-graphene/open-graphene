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
        .chain_id("1e9aea9e936607ca2cce053765c39c995c591ef23d85332ec63d6f06a0704f86")
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
