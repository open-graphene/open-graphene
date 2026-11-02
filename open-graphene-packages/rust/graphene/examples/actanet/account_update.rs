#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey, ChainApi};

// Change an account option and put it back, end to end on the node. We flip the memo key (used
// only to encrypt memos, never for spending or authority, so this cannot lock anyone out) to a
// throwaway key, check it changed, then restore the original. The active key signs.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("ACTANET_ACTIVE_WIF")?;
    let account = std::env::var("ACTANET_ACCOUNT").unwrap_or_else(|_| "actanet".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string(graphene::PUBLIC_KEY_PREFIX);

    // A valid key we will never use for anything, just to have something to flip the memo key to.
    let throwaway = PrivateKey::from_seed(b"og-account-update-probe")?
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
    let original = memo_key(&mut actanet, &id).await?;
    println!("memo key before: {original}");

    broadcast_memo_key(&mut actanet, &id, &throwaway, &wif, &public_key).await?;
    println!(
        "memo key now:    {} (changed: {})",
        memo_key(&mut actanet, &id).await?,
        memo_key(&mut actanet, &id).await? == throwaway
    );

    broadcast_memo_key(&mut actanet, &id, &original, &wif, &public_key).await?;
    println!(
        "memo key after:  {} (restored: {})",
        memo_key(&mut actanet, &id).await?,
        memo_key(&mut actanet, &id).await? == original
    );
    Ok(())
}

async fn memo_key(
    actanet: &mut ChainApi,
    id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(actanet
        .database()
        .get_account_by_id(id)
        .await?
        .options
        .memo_key)
}

async fn broadcast_memo_key(
    actanet: &mut ChainApi,
    id: &str,
    memo_key: &str,
    wif: &str,
    public_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signed = actanet
        .operations()
        .account_update(id)
        .memo_key(memo_key)
        .prepare()
        .await?
        .sign_with_wif(wif, public_key)?;
    actanet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    Ok(())
}
