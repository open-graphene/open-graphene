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
        .history()
        .account_history("swaplock")
        .limit(5)
        .offset(0)
        .subscribe()
        .await?;

    println!(
        "initial history page entries: {}",
        subscription.initial().items().len()
    );
    for item in subscription.initial().items() {
        println!(
            "initial history entry: {} block {} time {}",
            item.id.0, item.block_num, item.block_time
        );
    }
    println!("listening for account history updates; press Ctrl-C to stop");

    let mut updates_seen = 0_u64;
    loop {
        let updates = subscription.next_update().await?;
        for item in updates {
            updates_seen += 1;
            println!(
                "update #{updates_seen}: {} block {} time {}",
                item.id.0, item.block_num, item.block_time
            );
        }
    }
}
