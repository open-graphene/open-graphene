#[path = "common.rs"]
mod common;

use std::collections::HashSet;

use graphene::{Graphene, PrivateKey};

// Place a limit order and then cancel it, end to end on the node. We sell a tiny bit of ACTA for an
// absurd amount of another asset so the order just rests (it won't fill), then we find its id and
// cancel it.
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
    println!("new order id: {}", new_order.id.0);

    let signed = actanet
        .operations()
        .limit_order_cancel(&seller_id, &new_order.id.0)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    actanet
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("order {} cancelled", new_order.id.0);
    Ok(())
}
