use graphene::{Graphene, PrivateKey, SwaplockApi};

// Change a user asset's description and put it back, end to end on the node. We must be the asset's
// issuer. Description is free text in the options, so this is harmless and fully reversible.
const ASSET: &str = "1.3.102";

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
        .chain_id("9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let issuer = swaplock
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;
    let original = description(&mut swaplock).await?;
    println!("description before: {original:?}");

    broadcast_description(
        &mut swaplock,
        &issuer,
        "open-graphene asset_update probe",
        &wif,
        &public_key,
    )
    .await?;
    println!(
        "description now:    {:?}",
        description(&mut swaplock).await?
    );

    broadcast_description(&mut swaplock, &issuer, &original, &wif, &public_key).await?;
    let restored = description(&mut swaplock).await?;
    println!(
        "description after:  {restored:?} (restored: {})",
        restored == original
    );
    Ok(())
}

async fn description(swaplock: &mut SwaplockApi) -> Result<String, Box<dyn std::error::Error>> {
    Ok(swaplock
        .database()
        .get_asset_by_id(ASSET)
        .await?
        .options
        .description)
}

async fn broadcast_description(
    swaplock: &mut SwaplockApi,
    issuer: &str,
    description: &str,
    wif: &str,
    public_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signed = swaplock
        .operations()
        .asset_update(issuer, ASSET)
        .description(description)
        .prepare()
        .await?
        .sign_with_wif(wif, public_key)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    Ok(())
}
