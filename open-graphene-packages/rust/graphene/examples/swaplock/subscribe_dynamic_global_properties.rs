use std::time::{Duration, Instant};

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

    let initial = swaplock
        .database()
        .subscribe_dynamic_global_properties()
        .await?;
    println!(
        "subscribed at head block: {} {}",
        initial.head_block_number, initial.time
    );

    let started_at = Instant::now();
    let run_for = Duration::from_secs(60);
    let mut updates = 0_u64;

    while started_at.elapsed() < run_for {
        let update = swaplock
            .database()
            .next_dynamic_global_properties_update()
            .await?;
        updates += 1;
        println!(
            "update #{updates}: head block {} {} | last irreversible block {}",
            update.head_block_number, update.time, update.last_irreversible_block_num
        );
    }

    println!("received {updates} updates in {:?}", started_at.elapsed());

    Ok(())
}
