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

    let first_page = actanet
        .history()
        .account_history("actanet")
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
        let second_page = actanet
            .history()
            .account_history("actanet")
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
