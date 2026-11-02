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

    let orders = actanet.database().account_orders("actanet").get().await?;
    if orders.is_empty() {
        println!("open orders by name: none");
    }
    for order in orders {
        println!(
            "open order by name: {} seller {} for_sale {}",
            order.id.0, order.seller.0, order.for_sale
        );
    }

    let account = actanet
        .database()
        .account_by_name("actanet")
        .get()
        .await?;
    let orders = actanet
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
