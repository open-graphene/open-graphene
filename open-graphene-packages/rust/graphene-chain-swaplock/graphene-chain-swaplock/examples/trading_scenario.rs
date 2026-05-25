use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let result = graphene_chain_swaplock::run_trading_scenario_from_env()?;
    println!(
        "Trading scenario result: accounts {} ({}) / {} ({}), assets {} ({}) / {} ({}), order {} canceled",
        result.account_a,
        result.account_a_id,
        result.account_b,
        result.account_b_id,
        result.asset_a_symbol,
        result.asset_a_id,
        result.asset_b_symbol,
        result.asset_b_id,
        result.opened_order_id
    );
    Ok(())
}
