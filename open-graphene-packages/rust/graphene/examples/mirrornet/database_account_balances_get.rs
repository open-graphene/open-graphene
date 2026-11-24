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

    let balances = mirrornet
        .database()
        .account_balances("actanet", ["MBT"])
        .get()
        .await?;
    for balance in balances {
        println!("balance by name: {} {}", balance.amount, balance.asset_id.0);
    }

    let account = mirrornet
        .database()
        .account_by_name("actanet")
        .get()
        .await?;
    let asset = mirrornet.database().asset_by_symbol("MBT").get().await?;
    let balances = mirrornet
        .database()
        .account_balances_by_id(&account.id.0, [asset.id.0.as_str()])
        .get()
        .await?;
    for balance in balances {
        println!("balance by id: {} {}", balance.amount, balance.asset_id.0);
    }

    Ok(())
}
