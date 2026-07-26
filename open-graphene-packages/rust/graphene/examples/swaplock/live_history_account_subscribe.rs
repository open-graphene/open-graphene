use std::time::Duration;

use graphene::Graphene;

const WAIT_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let live = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("95acb1f01e4afd0dd8b61f99377911a2f0ac6e6420925e01630f1d3580c89758")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect_live()
        .await?;

    let history = live
        .history()?
        .account_history_by_id("1.2.100")
        .limit(5)
        .offset(0)
        .subscribe_timeout(WAIT_TIMEOUT)
        .await?;

    println!(
        "live account history initial page entries: {}",
        history.initial().items().len()
    );
    for item in history.initial().items() {
        println!(
            "live history initial entry: {} block {} time {}",
            item.id.0, item.block_num, item.block_time
        );
    }
    println!("registered typed live account history subscription on one websocket");

    Ok(())
}
