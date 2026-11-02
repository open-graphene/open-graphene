#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey};

// Read the raw order book for a market. To make sure the book is not empty, we first place a tiny
// resting order on the ACTA/OGA pair, then pull the book with get_limit_orders and check our order
// shows up, then clean up by cancelling.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("ACTANET_ACTIVE_WIF")?;
    let account = std::env::var("ACTANET_ACCOUNT").unwrap_or_else(|_| "actanet".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string(graphene::PUBLIC_KEY_PREFIX);

    let base = "1.3.0";
    let quote = "1.3.101";

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

    let signed = actanet
        .operations()
        .limit_order_create(&seller_id)
        .sell(1000, base)
        .receive(1_000_000_000, quote)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    actanet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    println!("limit order placed");

    let book = actanet
        .database()
        .limit_orders(base, quote)
        .limit(50)
        .get()
        .await?;
    println!("{base}/{quote} book has {} order(s)", book.len());

    let mine = book.iter().find(|order| order.seller.0 == seller_id);
    match mine {
        Some(order) => println!("found my order {} on the book", order.id.0),
        None => println!("my order is not in this page of the book"),
    }

    if let Some(order) = mine {
        let signed = actanet
            .operations()
            .limit_order_cancel(&seller_id, &order.id.0)
            .prepare()
            .await?
            .sign_with_wif(&wif, &public_key)?;
        actanet
            .network_broadcast()
            .broadcast_transaction(signed.transaction_json()?)
            .await?;
        println!("order {} cancelled", order.id.0);
    }
    Ok(())
}
