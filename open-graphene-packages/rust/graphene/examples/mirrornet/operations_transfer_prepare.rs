#[path = "common.rs"]
mod common;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        .from("mirrornet")
        .to("1.2.0")
        .amount_raw(1, "MBT")
        .fee_asset("MBT")
        .max_fee_raw(1_000_000)
        .prepare()
        .await?;

    println!("prepared transfer preview");
    println!("from: {}", prepared.from_id());
    println!("to: {}", prepared.to_id());
    println!("amount: {} {}", prepared.amount(), prepared.asset_id());
    println!(
        "fee: {} {}",
        prepared.fee().amount,
        prepared.fee().asset_id.0
    );
    println!("balance before: {}", prepared.balance_before());
    println!("balance after: {}", prepared.balance_after());
    println!("head block: {}", prepared.head_block_number());
    println!(
        "transaction operations: {}",
        prepared.transaction().operations.len()
    );

    Ok(())
}
