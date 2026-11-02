#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey, ChainApi};

// Change a user asset's description and put it back, end to end on the node. We must be the asset's
// issuer. Description is free text in the options, so this is harmless and fully reversible.
const ASSET: &str = "1.3.102";

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

    let issuer = actanet
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;
    let original = description(&mut actanet).await?;
    println!("description before: {original:?}");

    broadcast_description(
        &mut actanet,
        &issuer,
        "open-graphene asset_update probe",
        &wif,
        &public_key,
    )
    .await?;
    println!(
        "description now:    {:?}",
        description(&mut actanet).await?
    );

    broadcast_description(&mut actanet, &issuer, &original, &wif, &public_key).await?;
    let restored = description(&mut actanet).await?;
    println!(
        "description after:  {restored:?} (restored: {})",
        restored == original
    );
    Ok(())
}

async fn description(actanet: &mut ChainApi) -> Result<String, Box<dyn std::error::Error>> {
    Ok(actanet
        .database()
        .get_asset_by_id(ASSET)
        .await?
        .options
        .description)
}

async fn broadcast_description(
    actanet: &mut ChainApi,
    issuer: &str,
    description: &str,
    wif: &str,
    public_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signed = actanet
        .operations()
        .asset_update(issuer, ASSET)
        .description(description)
        .prepare()
        .await?
        .sign_with_wif(wif, public_key)?;
    actanet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    Ok(())
}
