#[path = "common.rs"]
mod common;

use std::collections::HashSet;
use std::time::Duration;

use graphene::{Graphene, PrivateKey};

// Place a resting limit order, then change it in place (reprice, add a bit more to sell, push the
// expiry out), then clean up by cancelling. The order sells a tiny bit of ACTA for an absurd amount
// of another asset so it never fills while we poke at it.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("ACTANET_ACTIVE_WIF")?;
    let account = std::env::var("ACTANET_ACCOUNT").unwrap_or_else(|_| "actanet".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string(graphene::PUBLIC_KEY_PREFIX);

    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let seller_id = actanet
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    let before: HashSet<String> = actanet
        .database()
        .account_orders_by_id(&seller_id)
        .get()
        .await?
        .into_iter()
        .map(|order| order.id.0)
        .collect();

    let signed = actanet
        .operations()
        .limit_order_create(&seller_id)
        .sell(1000, "1.3.0")
        .receive(1_000_000_000, "1.3.101")
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    actanet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    println!("limit order placed");

    let after = actanet
        .database()
        .account_orders_by_id(&seller_id)
        .get()
        .await?;
    let new_order = after
        .iter()
        .find(|order| !before.contains(&order.id.0))
        .ok_or("could not find the new order")?;
    let order_id = new_order.id.0.clone();
    println!("new order id: {order_id}");

    // Reprice, top the stake up by a touch, and give it another hour on the book.
    let signed = actanet
        .operations()
        .limit_order_update(&seller_id, &order_id)
        .new_price(1000, "1.3.0", 2_000_000_000, "1.3.101")
        .add_to_sell(500, "1.3.0")
        .extend_by(Duration::from_secs(3600))
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    actanet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    println!("order {order_id} updated");

    let signed = actanet
        .operations()
        .limit_order_cancel(&seller_id, &order_id)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    actanet
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("order {order_id} cancelled");
    Ok(())
}
