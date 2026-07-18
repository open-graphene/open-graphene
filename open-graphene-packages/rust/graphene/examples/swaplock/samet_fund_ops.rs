use graphene::{Graphene, PrivateKey};

// Exercise the SameT-fund family. create -> update -> delete is a reversible round-trip (delete
// reclaims the committed balance), so we broadcast it live and read the new fund id from the
// result. borrow and repay only validate as a pair in the same transaction (a flash loan), so we
// node-verify each at prepare() without broadcasting.
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
        .chain_id("f2491c85896bb49f936152d59b850ca05bb8e09d9027eb630267697ee483b05e")
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

    // 1. post a fund lending core, reading the new fund id from the result.
    let signed = swaplock
        .operations()
        .samet_fund_create(&id, CORE, 100_000)
        .fee_rate(10_000)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    let conf = swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let fund = conf.trx["operation_results"][0][1]
        .as_str()
        .ok_or("no fund id")?
        .to_string();
    println!("created samet fund {fund}");

    // 2. raise its fee rate in place.
    let signed = swaplock
        .operations()
        .samet_fund_update(&id, &fund)
        .fee_rate(20_000)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("raised the fund's fee rate");

    // 3. withdraw it, reclaiming the balance.
    let signed = swaplock
        .operations()
        .samet_fund_delete(&id, &fund)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("deleted the fund and reclaimed the balance");

    // Node-verify the flash-loan ops at prepare() (they only apply as a same-tx pair); not broadcast.
    swaplock
        .operations()
        .samet_fund_borrow(&id, "1.20.0")
        .amount(1, CORE)
        .prepare()
        .await?;
    swaplock
        .operations()
        .samet_fund_repay(&id, "1.20.0")
        .repay(1, CORE)
        .fund_fee(1, CORE)
        .prepare()
        .await?;
    println!("borrow / repay: node-priced (not broadcast)");
    Ok(())
}
