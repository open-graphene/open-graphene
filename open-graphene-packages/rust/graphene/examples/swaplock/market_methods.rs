use std::time::Duration;

use graphene::Graphene;

// Exercise the four market methods:
//   - history_api: get_market_history (OHLC candles), get_fill_order_history (recent trades)
//   - database_api (live): subscribe_to_market / unsubscribe_from_market (order-book stream)
// All run live against the node; a thin market may return empty history or no notice within the
// timeout, which still proves the calls round-trip.
const BASE: &str = "1.3.0"; // core
const QUOTE: &str = "1.3.102"; // a user asset
const WAIT: Duration = Duration::from_secs(8);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

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

    // get_market_history: hourly candles over a recent window.
    let candles = swaplock
        .history()
        .get_market_history(BASE, QUOTE)
        .bucket_seconds(3600)
        .range("2026-06-19T23:59:59", "2026-06-01T00:00:00")
        .get()
        .await?;
    println!("get_market_history: {} bucket(s)", candles.len());

    // get_fill_order_history: most recent trades.
    let fills = swaplock
        .history()
        .get_fill_order_history(BASE, QUOTE)
        .limit(10)
        .get()
        .await?;
    println!("get_fill_order_history: {} fill(s)", fills.len());

    // Live order-book subscription.
    let live = swaplock.into_live()?;
    let mut market = live.database().subscribe_to_market(BASE, QUOTE).await?;
    println!(
        "subscribed to market {}/{}",
        market.market().0,
        market.market().1
    );

    match market.next_timeout(WAIT).await {
        Ok(notice) => println!(
            "order-book update: {} top-level entries",
            notice.as_array().map_or(0, Vec::len)
        ),
        Err(_) => {
            println!("no order-book update within {WAIT:?} (quiet market) - subscription works")
        }
    }

    drop(market); // sends unsubscribe_from_market
    println!("unsubscribed (on drop)");
    Ok(())
}
