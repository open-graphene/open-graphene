use std::collections::HashSet;

use graphene::{Graphene, PrivateKey};

// Place a limit order and then cancel it, end to end on the node. We sell a tiny bit of BTS for an
// absurd amount of another asset so the order just rests (it won't fill), then we find its id and
// cancel it.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("SWAPLOCK_ACTIVE_WIF")?;
    let account = std::env::var("SWAPLOCK_ACCOUNT").unwrap_or_else(|_| "swaplock".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string("BTS");

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let seller_id = swaplock
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    let before: HashSet<String> = swaplock
        .database()
        .account_orders_by_id(&seller_id)
        .get()
        .await?
        .into_iter()
        .map(|order| order.id.0)
        .collect();

    let signed = swaplock
        .operations()
        .limit_order_create(&seller_id)
        .sell(1000, "1.3.0")
        .receive(1_000_000_000, "1.3.101")
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    println!("limit order placed");

    let after = swaplock
        .database()
        .account_orders_by_id(&seller_id)
        .get()
        .await?;
    let new_order = after
        .iter()
        .find(|order| !before.contains(&order.id.0))
        .ok_or("could not find the new order")?;
    println!("new order id: {}", new_order.id.0);

    let signed = swaplock
        .operations()
        .limit_order_cancel(&seller_id, &new_order.id.0)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("order {} cancelled", new_order.id.0);
    Ok(())
}
