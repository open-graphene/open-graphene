use graphene::Graphene;

// Exercise the last two operations: bid_collateral and committee_member_update_global_parameters.
//
// Both are node-verified at prepare() (the node prices/validates the op) without broadcasting:
// bid_collateral needs a globally-settled market-pegged asset, which swaplock does not have, and
// updating global parameters is council-only and would normally be wrapped in a proposal.
const CORE: &str = "1.3.0";
const MPA: &str = "1.3.102"; // stands in for a market-pegged asset's debt

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let account = std::env::var("SWAPLOCK_ACCOUNT").unwrap_or_else(|_| "swaplock".to_string());

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

    let id = swaplock
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    // bid_collateral: bid on a settled MPA's collateral.
    swaplock
        .operations()
        .bid_collateral(&id)
        .collateral(1, CORE)
        .debt_covered(1, MPA)
        .prepare()
        .await?;

    // committee_member_update_global_parameters: fetch the current parameters, then resubmit them
    // unchanged (the builder takes the whole ChainParameters; tweak fields for a real proposal).
    let parameters = swaplock
        .database()
        .global_properties()
        .get()
        .await?
        .parameters;
    swaplock
        .operations()
        .committee_member_update_global_parameters(parameters)
        .prepare()
        .await?;

    println!(
        "bid_collateral / committee_member_update_global_parameters: node-priced (not broadcast)"
    );
    Ok(())
}
