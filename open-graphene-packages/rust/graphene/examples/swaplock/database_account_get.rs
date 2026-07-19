use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("f990ce83af5cf2d55c180ca4bd4b34161ccff2b2f8cc7d4987eea9555153930e")
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
