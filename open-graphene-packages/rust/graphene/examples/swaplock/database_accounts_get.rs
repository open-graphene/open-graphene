use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
