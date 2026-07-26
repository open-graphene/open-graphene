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

    let mut subscription = swaplock
        .database()
        .dynamic_global_properties()
        .subscribe()
        .await?;
    println!(
        "subscribed at head block: {} {}",
        subscription.initial().head_block_number,
        subscription.initial().time
    );
    println!("listening for updates; press Ctrl-C to stop");

    let mut updates = 0_u64;

    loop {
        let update = subscription.next_update().await?;
        updates += 1;
        println!(
            "update #{updates}: head block {} {} | last irreversible block {}",
            update.head_block_number, update.time, update.last_irreversible_block_num
        );
    }
}
