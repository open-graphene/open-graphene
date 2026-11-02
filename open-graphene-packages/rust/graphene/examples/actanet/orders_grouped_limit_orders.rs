//! The `orders` API exposes a *grouped* view of a market's order book: instead of
//! every individual limit order, orders are bucketed into price bands. Each band
//! reports its price range and the total amount for sale inside it — handy for
//! drawing depth charts or market summaries cheaply.
//!
//! `group` is the band width in 0.01% units (e.g. 10 = 0.1%); valid widths come
//! from `tracked_groups()`.

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

    // Which group widths does this node track?
    let groups = actanet.orders().tracked_groups().get().await?;
    println!("tracked group widths (0.01%): {groups:?}");

    let Some(&group) = groups.first() else {
        println!("node tracks no order groups; nothing to show");
        return Ok(());
    };

    // Grouped order book for the ACTA/CNY market at the first tracked width.
    let book = actanet
        .orders()
        .grouped_limit_orders("ACTA", "CNY", group)
        .limit(10)
        .get()
        .await?;

    println!(
        "ACTA/CNY grouped orders (width {group}): {} bands",
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
