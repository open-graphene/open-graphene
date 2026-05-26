use std::env;
use std::error::Error;

use open_graphene_sdk_live::{GrapheneChainProfile, GrapheneLiveClient};

struct ExampleProfile;

impl GrapheneChainProfile for ExampleProfile {
    const CORE_ASSET_ID: &'static str = "1.3.0";
    const PUBLIC_KEY_PREFIX: &'static str = "BTS";
}

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = required_env("GRAPHENE_RPC_URL")?;
    let account_name = required_env("GRAPHENE_ACCOUNT_NAME")?;
    let asset_symbol = required_env("GRAPHENE_ASSET_SYMBOL")?;

    let mut client = GrapheneLiveClient::<ExampleProfile>::connect(&rpc_url)?;
    let head = client.head_block()?;
    let account_id = client.lookup_account_id(&account_name)?;
    let asset_id = client.lookup_asset_id(&asset_symbol)?;
    let balance = client.account_balance(&account_id, &asset_id)?;

    println!("Connected: chain_id={}", client.chain_id());
    println!("Head block: {} {} {}", head.number, head.id, head.time);
    println!("Account: {account_name} -> {account_id}");
    println!("Asset: {asset_symbol} -> {asset_id}");
    println!("Balance: {} {}", balance.amount, balance.asset_id);

    Ok(())
}

fn required_env(name: &'static str) -> Result<String, Box<dyn Error>> {
    env::var(name).map_err(|_| format!("missing required environment variable: {name}").into())
}
