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
        .account_balances("actanet", [])
        .subscribe()
        .await?;

    for balance in subscription.initial() {
        println!(
            "subscribed balance: {} owner {} asset {} amount {}",
            balance.id.0, balance.owner.0, balance.asset_type.0, balance.balance
        );
    }
    println!("listening for balance updates; press Ctrl-C to stop");

    let mut updates = 0_u64;

    loop {
        let changed_balances = subscription.next_update().await?;
        for balance in changed_balances {
            updates += 1;
            println!(
                "update #{updates}: {} owner {} asset {} amount {}",
                balance.id.0, balance.owner.0, balance.asset_type.0, balance.balance
            );
        }
    }
}
