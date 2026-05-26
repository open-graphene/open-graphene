use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use open_graphene_sdk_core::{AccountIdRef, AssetIdRef};
use serde_json::Value;

use crate::rpc::GrapheneRpc;

pub fn wait_for_limit_order(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    seller_id: &str,
    asset_a_id: &str,
    asset_b_id: &str,
) -> Result<String, Box<dyn Error>> {
    for _ in 0..20 {
        if let Some(order_id) = find_limit_order(rpc, api_id, seller_id, asset_a_id, asset_b_id)? {
            return Ok(order_id);
        }
        if let Some(order_id) = find_limit_order(rpc, api_id, seller_id, asset_b_id, asset_a_id)? {
            return Ok(order_id);
        }
        sleep(Duration::from_millis(500));
    }
    Err(format!("limit order for seller {seller_id} was not found").into())
}

pub fn find_limit_order(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    seller_id: &str,
    base_asset_id: &str,
    quote_asset_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    let seller_id = AccountIdRef::parse(seller_id)?;
    let base_asset_id = AssetIdRef::parse(base_asset_id)?;
    let quote_asset_id = AssetIdRef::parse(quote_asset_id)?;
    let orders = open_graphene_sdk_live::limit_orders(
        rpc.session_mut(),
        &base_asset_id,
        &quote_asset_id,
        100,
    )?;

    Ok(orders
        .into_iter()
        .find(|order| order.seller == seller_id)
        .map(|order| order.id.to_string()))
}

pub fn wait_for_order_gone(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    order_id: &str,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..20 {
        let result = rpc.get_objects(api_id, [order_id])?;
        if result
            .as_array()
            .and_then(|values| values.first())
            .is_some_and(Value::is_null)
        {
            println!("Order canceled: {order_id}");
            return Ok(());
        }
        sleep(Duration::from_millis(500));
    }
    Err(format!("order still exists after cancel: {order_id}").into())
}

fn ensure_database_api_id(rpc: &mut GrapheneRpc, api_id: u64) -> Result<(), Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != api_id {
        return Err(format!("database API id mismatch: expected {expected}, got {api_id}").into());
    }
    Ok(())
}
