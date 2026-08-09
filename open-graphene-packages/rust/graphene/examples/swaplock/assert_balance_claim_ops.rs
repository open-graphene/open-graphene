use graphene::{Graphene, PrivateKey};

// Exercise assert and balance_claim.
//
// assert is broadcast live with a predicate that holds (our own account name), so it applies as a
// harmless no-op. balance_claim needs a genesis/imported balance object, which this chain does not
// have, so it is node-verified at prepare() without broadcasting.
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

    // Live: assert that our account still has its name (a predicate that holds), so it applies as a
    // no-op and leaves nothing behind.
    let signed = swaplock
        .operations()
        .assert(&id)
        .assert_account_name(&id, &account)
        .require_auth(&id)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("asserted our account name (applied as a no-op)");

    // Node-verify balance_claim: there is no claimable balance object here to broadcast against.
    swaplock
        .operations()
        .balance_claim(&id, "1.15.0", &pk)
        .amount(1, CORE)
        .prepare()
        .await?;
    println!("balance_claim: node-priced (not broadcast)");
    Ok(())
}
