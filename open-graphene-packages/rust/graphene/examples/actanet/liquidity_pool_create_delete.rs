#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey};

// Open a liquidity pool and close it, end to end on the node. We pool ACTA (1.3.0) with one of our
// user assets, backed by another empty user asset for the shares, then read the new pool id and
// delete it. Create -> delete is a clean reversible round-trip.
const ASSET_A: &str = "1.3.0"; // must order before ASSET_B
const ASSET_B: &str = "1.3.102"; // an asset we issue
const SHARE_ASSET: &str = "1.3.100"; // an empty asset we issue, dedicated to the pool

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
        .liquidity_pool_create(&id)
        .assets(ASSET_A, ASSET_B)
        .share_asset(SHARE_ASSET)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    let confirmation = actanet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;

    // liquidity_pool_create reports a generic result: [tag, {new_objects: ["1.19.x"], ...}].
    let pool_id = confirmation.trx["operation_results"][0][1]["new_objects"][0]
        .as_str()
        .ok_or("no pool id in operation results")?
        .to_string();
    println!("pool created: {pool_id} ({ASSET_A}/{ASSET_B}, shares {SHARE_ASSET})");

    let signed = actanet
        .operations()
        .liquidity_pool_delete(&id, &pool_id)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    actanet
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("pool {pool_id} deleted");
    Ok(())
}
