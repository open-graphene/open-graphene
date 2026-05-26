use std::env;
use std::error::Error;

use graphene_chain_swaplock::{
    database_api::{asset_object, lookup_asset_id},
    SwaplockSession,
};

fn main() -> Result<(), Box<dyn Error>> {
    let asset_symbol = env::var("GRAPHENE_ASSET_SYMBOL").unwrap_or_else(|_| "BTS".to_string());

    let mut session = SwaplockSession::connect_from_env_or_default()?;
    let database_api_id = session.database_api_id();
    let asset_id = lookup_asset_id(session.rpc_mut(), database_api_id, &asset_symbol)?;
    let asset = asset_object(session.rpc_mut(), database_api_id, &asset_id)?
        .ok_or_else(|| format!("asset object {asset_id} was not found"))?;

    println!("Connected: database_api_id={database_api_id}");
    println!("Asset: {asset_symbol} -> {asset_id}");
    println!(
        "Asset object: id={} symbol={} precision={} issuer={}",
        asset.id.0, asset.symbol, asset.precision, asset.issuer.0
    );

    Ok(())
}
