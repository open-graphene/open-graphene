use graphene::Graphene;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
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
