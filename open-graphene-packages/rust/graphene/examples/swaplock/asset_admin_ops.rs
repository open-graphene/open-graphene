use graphene::{Graphene, PrivateKey};

// Exercise the rest of the asset operations. fund_fee_pool -> claim_pool is a reversible round-trip
// on one of our user assets and is broadcast live. The others are either irreversible (create,
// update_issuer) or only meaningful for a market-pegged asset (settle, global_settle, publish_feed,
// update_feed_producers, update_bitasset), so we node-verify them at prepare() without broadcasting.
const ASSET: &str = "1.3.102"; // a user asset we issue
const CORE: &str = "1.3.0";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("SWAPLOCK_ACTIVE_WIF")?;
    let account = std::env::var("SWAPLOCK_ACCOUNT").unwrap_or_else(|_| "swaplock".to_string());
    let pk = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string("BTS");

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("e80d8f63b598759059ca8f8627a6c9252bf6ae13ed404e1afbf4ae51b1781837")
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

    // Live reversible round-trip: fund the asset's fee pool, then claim it back.
    let signed = swaplock
        .operations()
        .asset_fund_fee_pool(&id, ASSET, 100_000)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("funded fee pool of {ASSET} with 100000 {CORE}");

    let signed = swaplock
        .operations()
        .asset_claim_pool(&id, ASSET, 100_000)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("claimed 100000 {CORE} back from the fee pool");

    // Node-verify the remaining ops at prepare() (the node prices/validates each); not broadcast.
    swaplock
        .operations()
        .asset_create(&id, "OGEXAMPLE", 5)
        .prepare()
        .await?;
    swaplock
        .operations()
        .asset_update_issuer(&id, ASSET, "1.2.0")
        .prepare()
        .await?;
    swaplock
        .operations()
        .asset_claim_fees(&id)
        .amount(1, ASSET)
        .prepare()
        .await?;
    swaplock
        .operations()
        .asset_settle(&id)
        .amount(1, ASSET)
        .prepare()
        .await?;
    swaplock
        .operations()
        .asset_global_settle(&id, ASSET)
        .settle_price(1, ASSET, 1, CORE)
        .prepare()
        .await?;
    swaplock
        .operations()
        .asset_update_feed_producers(&id, ASSET)
        .producers(["1.2.0"])
        .prepare()
        .await?;
    swaplock
        .operations()
        .asset_publish_feed(&id, ASSET)
        .settlement_price(1, ASSET, 1, CORE)
        .core_exchange_rate(1, ASSET, 1, CORE)
        .prepare()
        .await?;
    swaplock
        .operations()
        .asset_update_bitasset(&id, ASSET, CORE)
        .prepare()
        .await?;
    println!(
        "create / update_issuer / claim_fees / settle / global_settle / update_feed_producers / publish_feed / update_bitasset: node-priced (not broadcast)"
    );
    Ok(())
}
