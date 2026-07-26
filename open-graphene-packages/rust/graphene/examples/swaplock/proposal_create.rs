use std::time::Duration;

use graphene::{AccountId, Asset, AssetId, Graphene, Operation, PrivateKey, TransferOperation};

// Wrap an operation in a proposal, end to end on the node. We propose a tiny self-transfer and read
// the new proposal object id from the result. The proposal then waits for approval and expires on
// its own, so this leaves only a short-lived pending proposal behind.
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
        .chain_id("95acb1f01e4afd0dd8b61f99377911a2f0ac6e6420925e01630f1d3580c89758")
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

    // The operation to propose: a 1-unit BTS transfer from us to the committee account. It only
    // executes if the proposal is approved (we never approve it, so nothing actually moves). Fee is
    // left at zero; the proposal builder prices each wrapped op.
    let inner = Operation::transfer(TransferOperation {
        fee: Asset::new(0, AssetId("1.3.0".to_string())),
        from: AccountId(id.clone()),
        to: AccountId("1.2.0".to_string()),
        amount: Asset::new(1, AssetId("1.3.0".to_string())),
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
        .sign_with_wif(&wif, &public_key)?;
    let confirmation = swaplock
        .network_broadcast()
        .broadcast_transaction_with_callback(signed.transaction_json()?)
        .await?;

    let proposal_id = confirmation.trx["operation_results"][0][1]
        .as_str()
        .ok_or("no proposal id in operation results")?;
    println!("proposal created: {proposal_id} (wraps a transfer, awaits approval)");
    Ok(())
}
