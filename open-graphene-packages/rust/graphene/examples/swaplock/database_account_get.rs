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

    let account = swaplock
        .database()
        .account_by_name("swaplock")
        .get()
        .await?;
    println!("account by name: {} {}", account.id.0, account.name);

    let same_account = swaplock
        .database()
        .account_by_id(&account.id.0)
        .get()
        .await?;
    println!("account by id: {} {}", same_account.id.0, same_account.name);

    Ok(())
}
