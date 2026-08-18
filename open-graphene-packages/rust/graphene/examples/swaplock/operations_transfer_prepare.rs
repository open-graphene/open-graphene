use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
