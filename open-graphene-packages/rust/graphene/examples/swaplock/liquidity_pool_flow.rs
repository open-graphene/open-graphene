use graphene::{Graphene, PrivateKey};

// Full liquidity pool flow on the node: create a pool, fund ourselves, deposit both assets, tweak
// the fees, swap against the pool, then withdraw all our shares and delete the empty pool. Cleans
// up the issued asset at the end. Exercises deposit / exchange / update / withdraw end to end.
const ASSET_A: &str = "1.3.0";
const ASSET_B: &str = "1.3.102";
const SHARE_ASSET: &str = "1.3.100";

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

    // 1. create the pool, reading the new id from the result
    let signed = swaplock
        .operations()
        .liquidity_pool_create(&id)
        .assets(ASSET_A, ASSET_B)
        .share_asset(SHARE_ASSET)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    let conf = swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let pool = conf.trx["operation_results"][0][1]["new_objects"][0]
        .as_str()
        .ok_or("no pool id")?
        .to_string();
    println!("created pool {pool}");

    // 2. mint ourselves some of asset B to deposit
    let signed = swaplock
        .operations()
        .asset_issue(&id)
        .issue(1_000_000, ASSET_B)
        .to(&id)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;

    // 3. deposit both assets, receiving shares
    let signed = swaplock
        .operations()
        .liquidity_pool_deposit(&id, &pool)
        .amount_a(100_000, ASSET_A)
        .amount_b(100_000, ASSET_B)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("deposited");

    // 4. tweak the pool fees
    let signed = swaplock
        .operations()
        .liquidity_pool_update(&id, &pool)
        .taker_fee_percent(10)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("updated fees");

    // 5. swap a little A for B against the pool
    let signed = swaplock
        .operations()
        .liquidity_pool_exchange(&id, &pool)
        .sell(1_000, ASSET_A)
        .min_to_receive(1, ASSET_B)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("exchanged");

    // 6. withdraw all our shares, emptying our position
    let shares = swaplock
        .database()
        .account_balances_by_id(&id, [SHARE_ASSET])
        .get()
        .await?
        .first()
        .map_or(0, |asset| asset.amount);
    let signed = swaplock
        .operations()
        .liquidity_pool_withdraw(&id, &pool)
        .share_amount(shares, SHARE_ASSET)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("withdrew {shares} shares");

    // 7. delete the now-empty pool
    let signed = swaplock
        .operations()
        .liquidity_pool_delete(&id, &pool)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("deleted pool {pool}");

    // 8. burn the asset B we still hold, to leave it clean
    let held = swaplock
        .database()
        .account_balances_by_id(&id, [ASSET_B])
        .get()
        .await?
        .first()
        .map_or(0, |asset| asset.amount);
    if held > 0 {
        let signed = swaplock
            .operations()
            .asset_reserve(&id)
            .amount(held, ASSET_B)
            .prepare()
            .await?
            .sign_with_wif(&wif, &pk)?;
        swaplock
            .network_broadcast()
            .broadcast_transaction(signed.transaction_json()?)
            .await?;
        println!("reserved {held} {ASSET_B} back to clean up");
    }
    Ok(())
}
