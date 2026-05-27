use std::env;

use graphene::Graphene;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wif = env::var("SWAPLOCK_ACTIVE_WIF")?;
    let expected_public_key = env::var("SWAPLOCK_ACTIVE_PUBLIC_KEY")?;

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let prepared = swaplock
        .operations()
        .transfer()
        .from("swaplock")
        .to("1.2.0")
        .amount_raw(1, "BTS")
        .fee_asset("BTS")
        .max_fee_raw(1_000_000)
        .prepare()
        .await?;
    let signed = prepared.sign_with_wif(&wif, &expected_public_key)?;
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
