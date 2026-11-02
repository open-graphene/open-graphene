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
        .history()
        .account_history("actanet")
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
