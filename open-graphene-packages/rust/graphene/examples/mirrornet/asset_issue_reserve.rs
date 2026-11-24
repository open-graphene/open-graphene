#[path = "common.rs"]
mod common;

use graphene::{ChainApi, Graphene, PrivateKey};

// Mint a user asset we issue to ourselves, then burn it back, end to end on the node. Fully
// reversible: we issue a chunk of OGB79740068 (1.3.102) to our own account, check the balance went
// up, then reserve the same amount and check it came back down. We must be the asset's issuer.
const ASSET: &str = "1.3.102";
const AMOUNT: i64 = 100_000;

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

    let id = mirrornet
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    let before = balance(&mut mirrornet, &id).await?;
    println!("balance before: {before}");

    let signed = mirrornet
        .operations()
        .asset_issue(&id)
        .issue(AMOUNT, ASSET)
        .to(&id)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    mirrornet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let after_issue = balance(&mut mirrornet, &id).await?;
    println!(
        "balance after issue: {after_issue} (+{})",
        after_issue - before
    );

    let signed = mirrornet
        .operations()
        .asset_reserve(&id)
        .amount(AMOUNT, ASSET)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    mirrornet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let after_reserve = balance(&mut mirrornet, &id).await?;
    println!(
        "balance after reserve: {after_reserve} (back to start: {})",
        after_reserve == before
    );
    Ok(())
}

// Our current balance of ASSET, in raw units (0 if we hold none).
async fn balance(mirrornet: &mut ChainApi, id: &str) -> Result<i64, Box<dyn std::error::Error>> {
    let balances = mirrornet
        .database()
        .account_balances_by_id(id, [ASSET])
        .get()
        .await?;
    Ok(balances.first().map_or(0, |asset| asset.amount))
}
