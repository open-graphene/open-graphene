use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let requested = ["swaplock", "committee-account", "missing-account"];
    let accounts = swaplock.database().accounts(requested).get().await?;

    for (requested, account) in requested.iter().zip(accounts) {
        match account {
            Some(account) => println!("{requested}: {} {}", account.id.0, account.name),
            None => println!("{requested}: not found"),
        }
    }

    Ok(())
}
