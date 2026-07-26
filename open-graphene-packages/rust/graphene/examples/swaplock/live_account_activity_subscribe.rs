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

    let balances = live
        .database()
        .account_balances_by_id("1.2.100", ["1.3.0"])
        .subscribe_timeout(WAIT_TIMEOUT)
        .await?;
    if balances.initial().is_empty() {
        println!("live balance subscription initial: none");
    }
    for balance in balances.initial() {
        println!(
            "live balance subscription initial: {} owner {} asset {} amount {}",
            balance.id.0, balance.owner.0, balance.asset_type.0, balance.balance
        );
    }

    let orders = live
        .database()
        .account_orders_by_id("1.2.100")
        .subscribe_timeout(WAIT_TIMEOUT)
        .await?;
    if orders.initial().is_empty() {
        println!("live open orders subscription initial: none");
    }
    for order in orders.initial() {
        println!(
            "live open order subscription initial: {} seller {} for_sale {}",
            order.id.0, order.seller.0, order.for_sale
        );
    }

    println!("registered typed live account balance and order subscriptions on one websocket");

    Ok(())
}
