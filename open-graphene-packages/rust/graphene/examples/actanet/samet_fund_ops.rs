#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey};

// Exercise the SameT-fund family. create -> update -> delete is a reversible round-trip (delete
// reclaims the committed balance), so we broadcast it live and read the new fund id from the
// result. borrow and repay only validate as a pair in the same transaction (a flash loan), so we
// node-verify each at prepare() without broadcasting.
const CORE: &str = "1.3.0";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("ACTANET_ACTIVE_WIF")?;
    let account = std::env::var("ACTANET_ACCOUNT").unwrap_or_else(|_| "actanet".to_string());
    let pk = PrivateKey::from_wif(&wif)?
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

    // 1. post a fund lending core, reading the new fund id from the result.
    let signed = actanet
        .operations()
        .samet_fund_create(&id, CORE, 100_000)
        .fee_rate(10_000)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    let conf = actanet
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let fund = conf.trx["operation_results"][0][1]
        .as_str()
        .ok_or("no fund id")?
        .to_string();
    println!("created samet fund {fund}");

    // 2. raise its fee rate in place.
    let signed = actanet
        .operations()
        .samet_fund_update(&id, &fund)
        .fee_rate(20_000)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    actanet
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("raised the fund's fee rate");

    // 3. withdraw it, reclaiming the balance.
    let signed = actanet
        .operations()
        .samet_fund_delete(&id, &fund)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    actanet
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("deleted the fund and reclaimed the balance");

    // Node-verify the flash-loan ops at prepare() (they only apply as a same-tx pair); not broadcast.
    actanet
        .operations()
        .samet_fund_borrow(&id, "1.20.0")
        .amount(1, CORE)
        .prepare()
        .await?;
    actanet
        .operations()
        .samet_fund_repay(&id, "1.20.0")
        .repay(1, CORE)
        .fund_fee(1, CORE)
        .prepare()
        .await?;
    println!("borrow / repay: node-priced (not broadcast)");
    Ok(())
}
