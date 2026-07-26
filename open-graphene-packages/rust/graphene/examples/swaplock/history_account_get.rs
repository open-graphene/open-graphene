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
