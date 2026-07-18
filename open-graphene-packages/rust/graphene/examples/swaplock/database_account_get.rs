use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("f2491c85896bb49f936152d59b850ca05bb8e09d9027eb630267697ee483b05e")
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
