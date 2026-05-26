use std::env;
use std::error::Error;

use graphene_chain_swaplock::{
    database_api::{asset_dynamic_data_object, asset_object, lookup_asset_id},
    rpc::GrapheneRpc,
};

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = env::var("GRAPHENE_RPC_URL")
        .unwrap_or_else(|_| "wss://node02.swaplock.chainpool.online:8090".to_string());
    let asset_symbol = env::var("GRAPHENE_ASSET_SYMBOL").unwrap_or_else(|_| "BTS".to_string());

    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;
    let asset_id = lookup_asset_id(&mut rpc, database_api_id, &asset_symbol)?;
    let asset = asset_object(&mut rpc, database_api_id, &asset_id)?
        .ok_or_else(|| format!("asset object {asset_id} was not found"))?;
    let dynamic_data_id = asset.dynamic_asset_data_id.0.clone();
    let dynamic_data = asset_dynamic_data_object(&mut rpc, database_api_id, &dynamic_data_id)?
        .ok_or_else(|| format!("asset dynamic data object {dynamic_data_id} was not found"))?;

    println!("Connected: database_api_id={database_api_id}");
    println!("Asset: {asset_symbol} -> {asset_id}");
    println!("Dynamic data: {dynamic_data_id}");
    println!(
        "Asset dynamic data object: id={} current_supply={} confidential_supply={} accumulated_fees={} fee_pool={}",
        dynamic_data.id.0,
        dynamic_data.current_supply,
        dynamic_data.confidential_supply,
        dynamic_data.accumulated_fees,
        dynamic_data.fee_pool
    );

    Ok(())
}
