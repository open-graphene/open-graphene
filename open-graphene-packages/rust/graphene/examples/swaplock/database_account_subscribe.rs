use graphene::Graphene;

#[tokio::main]
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

    let mut subscription = swaplock
        .database()
        .account_by_name("swaplock")
        .subscribe()
        .await?;
    println!(
        "subscribed to account: {} {}",
        subscription.initial().id.0,
        subscription.initial().name
    );
    println!("listening for account updates; press Ctrl-C to stop");

    let mut updates = 0_u64;

    loop {
        let update = subscription.next_update().await?;
        updates += 1;
        println!("update #{updates}: {} {}", update.id.0, update.name);
    }
}
