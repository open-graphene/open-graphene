#[path = "common.rs"]
mod common;

use graphene::Graphene;

// Two read-only RPC queries: list the chain's assets and look up account names. Both paginate from
// a lower bound, so this is also how you would drive asset discovery or account autocomplete.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    println!("assets:");
    for asset in actanet.database().list_assets("").limit(10).get().await? {
        println!(
            "  {} {} precision {}",
            asset.id.0, asset.symbol, asset.precision
        );
    }

    println!("accounts:");
    for (name, id) in actanet
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
