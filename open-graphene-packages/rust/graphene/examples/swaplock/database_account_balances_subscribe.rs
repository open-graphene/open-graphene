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

    let mut subscription = swaplock
        .database()
        .account_balances("swaplock", [])
        .subscribe()
        .await?;

    for balance in subscription.initial() {
        println!(
            "subscribed balance: {} owner {} asset {} amount {}",
            balance.id.0, balance.owner.0, balance.asset_type.0, balance.balance
        );
    }
    println!("listening for balance updates; press Ctrl-C to stop");

    let mut updates = 0_u64;

    loop {
        let changed_balances = subscription.next_update().await?;
        for balance in changed_balances {
            updates += 1;
            println!(
                "update #{updates}: {} owner {} asset {} amount {}",
                balance.id.0, balance.owner.0, balance.asset_type.0, balance.balance
            );
        }
    }
}
