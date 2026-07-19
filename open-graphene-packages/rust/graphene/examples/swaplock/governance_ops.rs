use std::time::Duration;

use graphene::{AccountId, Asset, AssetId, Graphene, Operation, PrivateKey, TransferOperation};

// Exercise the governance operations.
//
// proposal create -> delete is a reversible round-trip (the wrapped op never executes), so we
// broadcast it live and read the new proposal id from the result. The rest are privileged or
// stake-heavy (committee, witness, worker) or carry an opaque payload (custom), so we node-verify
// them at prepare() without broadcasting.
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
        .chain_id("f990ce83af5cf2d55c180ca4bd4b34161ccff2b2f8cc7d4987eea9555153930e")
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

    // Live: propose a tiny self-transfer, read the new proposal id, then drop it before it executes.
    let inner = Operation::transfer(TransferOperation {
        fee: Asset::new(0, AssetId(CORE.to_string())),
        from: AccountId(id.clone()),
        to: AccountId("1.2.0".to_string()),
        amount: Asset::new(1, AssetId(CORE.to_string())),
        memo: None,
        extensions: vec![],
    });
    let signed = swaplock
        .operations()
        .proposal_create(&id)
        .propose(inner)
        .expiration(Duration::from_secs(3600))
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    let conf = swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;
    let proposal = conf.trx["operation_results"][0][1]
        .as_str()
        .ok_or("no proposal id")?
        .to_string();
    println!("created proposal {proposal}");

    let signed = swaplock
        .operations()
        .proposal_delete(&id, &proposal)
        .prepare()
        .await?
        .sign_with_wif(&wif, &pk)?;
    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("deleted the proposal");

    // Node-verify the rest at prepare() (privileged / stake-heavy / opaque payload); not broadcast.
    swaplock
        .operations()
        .proposal_update(&id, &proposal)
        .approve_active(&id)
        .prepare()
        .await?;
    swaplock
        .operations()
        .committee_member_create(&id)
        .url("https://example.test/committee")
        .prepare()
        .await?;
    swaplock
        .operations()
        .committee_member_update("1.5.0", &id)
        .url("https://example.test/committee/v2")
        .prepare()
        .await?;
    swaplock
        .operations()
        .witness_create(&id, &pk)
        .url("https://example.test/witness")
        .prepare()
        .await?;
    swaplock
        .operations()
        .witness_update("1.6.1", &id)
        .signing_key(&pk)
        .prepare()
        .await?;
    swaplock
        .operations()
        .worker_create(&id, "example worker")
        .daily_pay(100_000)
        .work_period("2026-07-01T00:00:00", "2026-08-01T00:00:00")
        .vesting(7)
        .prepare()
        .await?;
    swaplock
        .operations()
        .custom(&id)
        .id(42)
        .data(vec![1u8, 2, 3])
        .prepare()
        .await?;
    println!(
        "proposal_update / committee_member_create / committee_member_update / witness_create / witness_update / worker_create / custom: node-priced (not broadcast)"
    );
    Ok(())
}
