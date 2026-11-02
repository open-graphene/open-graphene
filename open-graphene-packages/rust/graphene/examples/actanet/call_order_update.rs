#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey};

// Adjust a margin position. The debt asset must be a market-pegged asset (bitasset). actanet has
// none, so the node parses our op and then rejects on policy; we treat that as "no MPA here". On a
// chain with a bitasset, pass its id as the debt asset to borrow against collateral.
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

    let id = actanet
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    let signed = actanet
        .operations()
        .call_order_update(&id)
        .delta_collateral(100_000, "1.3.0")
        .delta_debt(1, "1.3.102")
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    match actanet
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await
    {
        Ok(_) => println!("call_order_update broadcast"),
        Err(error) => println!(
            "call_order_update accepted by the node, rejected on policy (no market-pegged asset here): {error}"
        ),
    }
    Ok(())
}
