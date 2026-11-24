#[path = "common.rs"]
mod common;

use std::env;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let wif = env::var("MIRRORNET_ACTIVE_WIF")?;
    let from_account = env::var("MIRRORNET_ACCOUNT")?;
    let to_account = env::var("MIRRORNET_TO_ACCOUNT").unwrap_or_else(|_| "1.2.0".to_string());

    let mut mirrornet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .mirrornet()
        .connect()
        .await?;

    let prepared = mirrornet
        .operations()
        .transfer()
        .from(&from_account)
        .to(&to_account)
        .amount_decimal("9".to_string(), "MBT")
        .fee_asset("MBT")
        .max_fee_raw(1_000_000)
        .prepare()
        .await?;
    let signed = mirrornet
        .operations()
        .sign_transfer_with_wif(prepared, &wif)
        .await?;

    println!("submitting signed transfer without callback confirmation");
    println!("from: {}", signed.from_id());
    println!("to: {}", signed.to_id());
    println!("amount: {} {}", signed.amount(), signed.asset_id());
    println!("fee: {} {}", signed.fee().amount, signed.fee().asset_id.0);
    println!("head block: {}", signed.head_block_number());
    println!(
        "signatures: {}",
        signed.signed_transaction().signatures.len()
    );

    let receipt = mirrornet
        .network_broadcast()
        .broadcast_signed_transfer(signed)
        .await?;

    println!("submitted: node accepted broadcast_transaction request");
    println!(
        "transaction json operations: {}",
        receipt.transaction()["operations"]
            .as_array()
            .map_or(0, Vec::len)
    );

    Ok(())
}
