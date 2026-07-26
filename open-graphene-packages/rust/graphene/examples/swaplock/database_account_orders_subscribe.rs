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

    let mut subscription = swaplock
        .database()
        .account_orders("swaplock")
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
