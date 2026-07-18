use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("f2491c85896bb49f936152d59b850ca05bb8e09d9027eb630267697ee483b05e")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let orders = swaplock.database().account_orders("swaplock").get().await?;
    if orders.is_empty() {
        println!("open orders by name: none");
    }
    for order in orders {
        println!(
            "open order by name: {} seller {} for_sale {}",
            order.id.0, order.seller.0, order.for_sale
        );
    }

    let account = swaplock
        .database()
        .account_by_name("swaplock")
        .get()
        .await?;
    let orders = swaplock
        .database()
        .account_orders_by_id(&account.id.0)
        .get()
        .await?;
    if orders.is_empty() {
        println!("open orders by id: none");
    }
    for order in orders {
        println!(
            "open order by id: {} seller {} for_sale {}",
            order.id.0, order.seller.0, order.for_sale
        );
    }

    Ok(())
}
