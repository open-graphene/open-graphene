//! The `orders` API exposes a *grouped* view of a market's order book: instead of
//! every individual limit order, orders are bucketed into price bands. Each band
//! reports its price range and the total amount for sale inside it — handy for
//! drawing depth charts or market summaries cheaply.
//!
//! `group` is the band width in 0.01% units (e.g. 10 = 0.1%); valid widths come
//! from `tracked_groups()`.

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    // Which group widths does this node track?
    let groups = swaplock.orders().tracked_groups().get().await?;
    println!("tracked group widths (0.01%): {groups:?}");

    let Some(&group) = groups.first() else {
        println!("node tracks no order groups; nothing to show");
        return Ok(());
    };

    // Grouped order book for the BTS/CNY market at the first tracked width.
    let book = swaplock
        .orders()
        .grouped_limit_orders("BTS", "CNY", group)
        .limit(10)
        .get()
        .await?;

    println!(
        "BTS/CNY grouped orders (width {group}): {} bands",
        book.len()
    );
    for band in &book {
        println!(
            "  for_sale={:>15}  base_id={}  quote_id={}",
            band.total_for_sale, band.min_price.base.asset_id.0, band.min_price.quote.asset_id.0
        );
    }

    Ok(())
}
