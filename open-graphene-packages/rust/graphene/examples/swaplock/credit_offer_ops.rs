use graphene::{Graphene, PrivateKey};

// Exercise the credit-offer family. create -> update -> delete is a reversible round-trip (delete
// reclaims the committed balance), so we broadcast it live and read the new offer id from the
// result. accept / deal_repay / deal_update act on a borrower's open deal, which needs a second
// party, so we node-verify those at prepare() without broadcasting.
const CORE: &str = "1.3.0";
const COLLATERAL: &str = "1.3.102"; // a user asset, taken as collateral

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
        .chain_id("9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9")
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

    // 1. post a tiny offer lending core against core, reading the new offer id from the result.
    let signed = swaplock
        .operations()
        .credit_offer_create(&id, CORE, 100_000)
        .fee_rate(10_000)
        .accept_collateral(COLLATERAL, 1, CORE, 1, COLLATERAL)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    let conf = swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let offer = conf.trx["operation_results"][0][1]
        .as_str()
        .ok_or("no offer id")?
        .to_string();
    println!("created credit offer {offer}");

    // 2. raise its rate in place.
    let signed = swaplock
        .operations()
        .credit_offer_update(&id, &offer)
        .fee_rate(20_000)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("raised the offer's fee rate");

    // 3. withdraw it, reclaiming the balance.
    let signed = swaplock
        .operations()
        .credit_offer_delete(&id, &offer)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("deleted the offer and reclaimed the balance");

    // Node-verify the borrower-side ops at prepare() (they need a counterpart deal); not broadcast.
    swaplock
        .operations()
        .credit_offer_accept(&id, "1.21.0")
        .borrow(1, CORE)
        .collateral(2, CORE)
        .prepare()
        .await?;
    swaplock
        .operations()
        .credit_deal_repay(&id, "1.22.0")
        .repay(1, CORE)
        .credit_fee(1, CORE)
        .prepare()
        .await?;
    swaplock
        .operations()
        .credit_deal_update(&id, "1.22.0", 1)
        .prepare()
        .await?;
    println!("accept / deal_repay / deal_update: node-priced (not broadcast)");
    Ok(())
}
