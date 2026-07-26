use std::env;
use std::time::Duration;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let wif = env::var("SWAPLOCK_ACTIVE_WIF")?;
    let from_account = env::var("SWAPLOCK_ACCOUNT")?;
    let to_account = env::var("SWAPLOCK_TO_ACCOUNT").unwrap_or_else(|_| "1.2.0".to_string());

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

    let prepared = swaplock
        .operations()
        .transfer()
        .from(&from_account)
        .to(&to_account)
        .amount_raw(1, "BTS")
        .fee_asset("BTS")
        .max_fee_raw(1_000_000)
        .prepare()
        .await?;
    let signed = swaplock
        .operations()
        .sign_transfer_with_wif(prepared, &wif)
        .await?;

    println!("broadcasting signed transfer with callback confirmation");
    println!("from: {}", signed.from_id());
    println!("to: {}", signed.to_id());
    println!("amount: {} {}", signed.amount(), signed.asset_id());
    println!("fee: {} {}", signed.fee().amount, signed.fee().asset_id.0);
    println!("head block: {}", signed.head_block_number());
    println!(
        "signatures: {}",
        signed.signed_transaction().signatures.len()
    );

    let confirmation = swaplock
        .network_broadcast()
        .broadcast_signed_transfer_with_callback_timeout(signed, Duration::from_secs(30))
        .await?;

    println!("confirmed: block {}", confirmation.block_num());
    println!("transaction id: {}", confirmation.id());
    println!("trx_num: {}", confirmation.trx_num());
    println!(
        "operation results: {}",
        confirmation.transaction()["operation_results"]
            .as_array()
            .map_or(0, Vec::len)
    );

    Ok(())
}
