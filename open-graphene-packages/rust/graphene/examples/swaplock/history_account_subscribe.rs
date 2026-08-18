use graphene::Graphene;

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
