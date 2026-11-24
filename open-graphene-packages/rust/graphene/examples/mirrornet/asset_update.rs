#[path = "common.rs"]
mod common;

use graphene::{ChainApi, Graphene, PrivateKey};

// Change a user asset's description and put it back, end to end on the node. We must be the asset's
// issuer. Description is free text in the options, so this is harmless and fully reversible.
const ASSET: &str = "1.3.102";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("MIRRORNET_ACTIVE_WIF")?;
    let account = std::env::var("MIRRORNET_ACCOUNT").unwrap_or_else(|_| "actanet".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string(graphene::PUBLIC_KEY_PREFIX);

    let mut mirrornet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .mirrornet()
        .connect()
        .await?;

    let issuer = mirrornet
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;
    let original = description(&mut mirrornet).await?;
    println!("description before: {original:?}");

    broadcast_description(
        &mut mirrornet,
        &issuer,
        "open-graphene asset_update probe",
        &wif,
        &public_key,
    )
    .await?;
    println!(
        "description now:    {:?}",
        description(&mut mirrornet).await?
    );

    broadcast_description(&mut mirrornet, &issuer, &original, &wif, &public_key).await?;
    let restored = description(&mut mirrornet).await?;
    println!(
        "description after:  {restored:?} (restored: {})",
        restored == original
    );
    Ok(())
}

async fn description(mirrornet: &mut ChainApi) -> Result<String, Box<dyn std::error::Error>> {
    Ok(mirrornet
        .database()
        .get_asset_by_id(ASSET)
        .await?
        .options
        .description)
}

async fn broadcast_description(
    mirrornet: &mut ChainApi,
    issuer: &str,
    description: &str,
    wif: &str,
    public_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signed = mirrornet
        .operations()
        .asset_update(issuer, ASSET)
        .description(description)
        .prepare()
        .await?
        .sign_with_wif(wif, public_key)?;
    mirrornet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    Ok(())
}
