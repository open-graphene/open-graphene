use graphene::{Graphene, PrivateKey, SwaplockApi};

// Change an account option and put it back, end to end on the node. We flip the memo key (used
// only to encrypt memos, never for spending or authority, so this cannot lock anyone out) to a
// throwaway key, check it changed, then restore the original. The active key signs.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("SWAPLOCK_ACTIVE_WIF")?;
    let account = std::env::var("SWAPLOCK_ACCOUNT").unwrap_or_else(|_| "swaplock".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string("BTS");

    // A valid key we will never use for anything, just to have something to flip the memo key to.
    let throwaway = PrivateKey::from_seed(b"og-account-update-probe")?
        .to_public_key()
        .to_prefixed_string("BTS");

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("1e9aea9e936607ca2cce053765c39c995c591ef23d85332ec63d6f06a0704f86")
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
    let original = memo_key(&mut swaplock, &id).await?;
    println!("memo key before: {original}");

    broadcast_memo_key(&mut swaplock, &id, &throwaway, &wif, &public_key).await?;
    println!(
        "memo key now:    {} (changed: {})",
        memo_key(&mut swaplock, &id).await?,
        memo_key(&mut swaplock, &id).await? == throwaway
    );

    broadcast_memo_key(&mut swaplock, &id, &original, &wif, &public_key).await?;
    println!(
        "memo key after:  {} (restored: {})",
        memo_key(&mut swaplock, &id).await?,
        memo_key(&mut swaplock, &id).await? == original
    );
    Ok(())
}

async fn memo_key(
    swaplock: &mut SwaplockApi,
    id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(swaplock
        .database()
        .get_account_by_id(id)
        .await?
        .options
        .memo_key)
}

async fn broadcast_memo_key(
    swaplock: &mut SwaplockApi,
    id: &str,
    memo_key: &str,
    wif: &str,
    public_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signed = swaplock
        .operations()
        .account_update(id)
        .memo_key(memo_key)
        .prepare()
        .await?
        .sign_with_wif(wif, public_key)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    Ok(())
}
