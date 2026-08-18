use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("1e9aea9e936607ca2cce053765c39c995c591ef23d85332ec63d6f06a0704f86")
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
