use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("3b4f346481a66146d732eb22a43b5956134e5bea6124b379033589146036cbbf")
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
