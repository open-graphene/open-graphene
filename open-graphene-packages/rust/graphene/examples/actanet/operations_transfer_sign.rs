#[path = "common.rs"]
mod common;

use std::env;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let wif = env::var("ACTANET_ACTIVE_WIF")?;
    let from_account = env::var("ACTANET_ACCOUNT")?;

    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let prepared = actanet
        .operations()
        .transfer()
        .from(&from_account)
        .to("1.2.0")
        .amount_raw(1, "ACTA")
        .fee_asset("ACTA")
        .max_fee_raw(1_000_000)
        .prepare()
        .await?;
    let signed = actanet
        .operations()
        .sign_transfer_with_wif(prepared, &wif)
        .await?;
    let transaction_json = signed.transaction_json()?;

    println!("signed transfer preview");
    println!("from: {}", signed.from_id());
    println!("to: {}", signed.to_id());
    println!("amount: {} {}", signed.amount(), signed.asset_id());
    println!("fee: {} {}", signed.fee().amount, signed.fee().asset_id.0);
    println!("head block: {}", signed.head_block_number());
    println!(
        "signatures: {}",
        signed.signed_transaction().signatures.len()
    );
    println!(
        "transaction json operations: {}",
        transaction_json["operations"]
            .as_array()
            .map_or(0, Vec::len)
    );

    Ok(())
}
