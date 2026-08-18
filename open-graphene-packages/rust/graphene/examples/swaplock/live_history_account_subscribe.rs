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
        .chain_id("f7dc1f352cb6b8aef3d14a4aab8cb5592e440c79f8634252e327b40610b7784d")
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
