#[path = "common.rs"]
mod common;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let mut subscription = actanet
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
