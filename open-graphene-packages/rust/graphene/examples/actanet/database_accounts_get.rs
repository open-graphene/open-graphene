#[path = "common.rs"]
mod common;

use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let requested = ["actanet", "committee-account", "missing-account"];
    let accounts = actanet.database().accounts(requested).get().await?;

    for (requested, account) in requested.iter().zip(accounts) {
        match account {
            Some(account) => println!("{requested}: {} {}", account.id.0, account.name),
            None => println!("{requested}: not found"),
        }
    }

    Ok(())
}
