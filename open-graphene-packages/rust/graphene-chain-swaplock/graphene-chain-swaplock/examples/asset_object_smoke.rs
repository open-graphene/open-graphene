use std::env;
use std::error::Error;

use graphene_chain_swaplock::{
    database_api::{asset_object, lookup_asset_id},
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

    println!("Connected: database_api_id={database_api_id}");
    println!("Asset: {asset_symbol} -> {asset_id}");
    println!(
        "Asset object: id={} symbol={} precision={} issuer={}",
        asset.id.0, asset.symbol, asset.precision, asset.issuer.0
    );

    Ok(())
}
