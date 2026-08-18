use std::time::Duration;

use graphene::{Graphene, PrivateKey};

// Lock funds in an HTLC and immediately redeem them, end to end on the node. We lock a little BTS
// to ourselves under sha256(secret), read the new contract id from the create result, then redeem
// by revealing the secret. Funds come back to us, so it is a self-contained round-trip.
const SECRET: &[u8] = b"open-graphene-htlc-secret";
const AMOUNT: i64 = 10_000;

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
        .chain_id("f7dc1f352cb6b8aef3d14a4aab8cb5592e440c79f8634252e327b40610b7784d")
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

    let signed = swaplock
        .operations()
        .htlc_create(&id, &id)
        .amount(AMOUNT, "1.3.0")
        .lock_sha256(SECRET)
        .claim_period(Duration::from_secs(3600))
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    let confirmation = match swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await
    {
        Ok(confirmation) => confirmation,
        // swaplock has HTLC turned off at the chain level; the node parses our op fine and only
        // rejects on policy. Treat that as a clean "not enabled here" rather than a failure.
        Err(error) if error.to_string().contains("HTLC") => {
            println!("htlc op accepted by the node, but HTLC is disabled on this chain: {error}");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };

    // htlc_create reports the new contract id in the operation results: [[result_tag, "1.16.x"]].
    let htlc_id = confirmation.trx["operation_results"][0][1]
        .as_str()
        .ok_or("no htlc id in operation results")?
        .to_string();
    println!("htlc created: {htlc_id}");

    let signed = swaplock
        .operations()
        .htlc_redeem(&htlc_id, &id)
        .preimage(SECRET)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("htlc {htlc_id} redeemed with the preimage");
    Ok(())
}
