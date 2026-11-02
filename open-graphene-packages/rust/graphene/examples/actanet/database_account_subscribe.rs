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

    let mut subscription = actanet
        .database()
        .account_by_name("actanet")
        .subscribe()
        .await?;
    println!(
        "subscribed to account: {} {}",
        subscription.initial().id.0,
        subscription.initial().name
    );
    println!("listening for account updates; press Ctrl-C to stop");

    let mut updates = 0_u64;

    loop {
        let update = subscription.next_update().await?;
        updates += 1;
        println!("update #{updates}: {} {}", update.id.0, update.name);
    }
}
