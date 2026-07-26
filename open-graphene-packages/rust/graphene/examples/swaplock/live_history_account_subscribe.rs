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
        .chain_id("9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9")
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
