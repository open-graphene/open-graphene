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

    let mut subscription = actanet
        .database()
        .account_orders("actanet")
        .subscribe()
        .await?;

    if subscription.initial().is_empty() {
        println!("subscribed open orders: none");
    }
    for order in subscription.initial() {
        println!(
            "subscribed open order: {} seller {} for_sale {}",
            order.id.0, order.seller.0, order.for_sale
        );
    }
    println!("listening for order updates; press Ctrl-C to stop");

    let mut updates = 0_u64;

    loop {
        let changed_orders = subscription.next_update().await?;
        for order in changed_orders {
            updates += 1;
            println!(
                "update #{updates}: {} seller {} for_sale {}",
                order.id.0, order.seller.0, order.for_sale
            );
        }
    }
}
