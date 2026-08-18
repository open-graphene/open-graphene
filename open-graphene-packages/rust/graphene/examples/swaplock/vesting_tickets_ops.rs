use graphene::{Graphene, PrivateKey};

// Exercise vesting balances, withdraw permissions and stake tickets.
//
// Reversible round-trips broadcast live: an instant vesting balance create -> withdraw, and a
// withdraw permission create -> update -> delete (no funds move until a claim). The remaining ops
// are node-verified at prepare(): a claim needs the authorised party, and a ticket locks funds with
// a cooldown that does not cleanly reverse.
const CORE: &str = "1.3.0";
const AUTHORIZED: &str = "1.2.0"; // committee account, used as the authorised withdrawer

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

    // Vesting: lock funds under an instant policy (immediately vested), reading the new id, then
    // withdraw them straight back.
    let signed = swaplock
        .operations()
        .vesting_balance_create(&id, &id)
        .amount(10_000, CORE)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    let conf = swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let balance = conf.trx["operation_results"][0][1]
        .as_str()
        .ok_or("no vesting balance id")?
        .to_string();
    println!("created vesting balance {balance}");

    let signed = swaplock
        .operations()
        .vesting_balance_withdraw(&balance, &id)
        .amount(10_000, CORE)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("withdrew the vested funds back");

    // Withdraw permission: authorise the committee account to pull from us, tweak it, then revoke.
    let signed = swaplock
        .operations()
        .withdraw_permission_create(&id, AUTHORIZED)
        .limit(1_000, CORE)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    let conf = swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let permission = conf.trx["operation_results"][0][1]
        .as_str()
        .ok_or("no permission id")?
        .to_string();
    println!("created withdraw permission {permission}");

    let signed = swaplock
        .operations()
        .withdraw_permission_update(&id, AUTHORIZED, &permission)
        .limit(2_000, CORE)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("raised the permission's limit");

    let signed = swaplock
        .operations()
        .withdraw_permission_delete(&id, AUTHORIZED, &permission)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("revoked the withdraw permission");

    // Node-verify the rest at prepare() (claim needs the authorised party; tickets lock with a
    // cooldown); not broadcast.
    swaplock
        .operations()
        .withdraw_permission_claim(&permission, &id, AUTHORIZED)
        .amount(1_000, CORE)
        .prepare()
        .await?;
    swaplock
        .operations()
        .ticket_create(&id, 1)
        .amount(10_000, CORE)
        .prepare()
        .await?;
    swaplock
        .operations()
        .ticket_update("1.18.0", &id, 2)
        .prepare()
        .await?;
    println!("claim / ticket_create / ticket_update: node-priced (not broadcast)");
    Ok(())
}
