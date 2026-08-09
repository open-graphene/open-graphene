use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("e80d8f63b598759059ca8f8627a6c9252bf6ae13ed404e1afbf4ae51b1781837")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let balances = swaplock
        .database()
        .account_balances("swaplock", ["BTS"])
        .get()
        .await?;
    for balance in balances {
        println!("balance by name: {} {}", balance.amount, balance.asset_id.0);
    }

    let account = swaplock
        .database()
        .account_by_name("swaplock")
        .get()
        .await?;
    let asset = swaplock.database().asset_by_symbol("BTS").get().await?;
    let balances = swaplock
        .database()
        .account_balances_by_id(&account.id.0, [asset.id.0.as_str()])
        .get()
        .await?;
    for balance in balances {
        println!("balance by id: {} {}", balance.amount, balance.asset_id.0);
    }

    Ok(())
}
