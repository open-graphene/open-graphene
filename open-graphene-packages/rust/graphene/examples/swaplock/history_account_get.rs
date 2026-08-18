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

    let first_page = swaplock
        .history()
        .account_history("swaplock")
        .limit(5)
        .offset(0)
        .get()
        .await?;

    println!(
        "first history page entries: {} offset {}",
        first_page.items().len(),
        first_page.offset()
    );
    for item in first_page.items() {
        println!(
            "history entry: {} block {} time {}",
            item.id.0, item.block_num, item.block_time
        );
    }

    if let Some(next_offset) = first_page.next_offset() {
        let second_page = swaplock
            .history()
            .account_history("swaplock")
            .limit(5)
            .offset(next_offset)
            .get()
            .await?;

        println!(
            "second history page entries: {} offset {}",
            second_page.items().len(),
            second_page.offset()
        );
        for item in second_page.items() {
            println!(
                "history entry: {} block {} time {}",
                item.id.0, item.block_num, item.block_time
            );
        }
    } else {
        println!("no second history page available");
    }

    Ok(())
}
