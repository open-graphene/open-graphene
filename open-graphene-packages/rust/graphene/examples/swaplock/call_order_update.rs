use graphene::{Graphene, PrivateKey};

// Adjust a margin position. The debt asset must be a market-pegged asset (bitasset). swaplock has
// none, so the node parses our op and then rejects on policy; we treat that as "no MPA here". On a
// chain with a bitasset, pass its id as the debt asset to borrow against collateral.
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

    let id = swaplock
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    let signed = swaplock
        .operations()
        .call_order_update(&id)
        .delta_collateral(100_000, "1.3.0")
        .delta_debt(1, "1.3.102")
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    match swaplock
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
