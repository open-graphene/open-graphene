use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use serde_json::{json, Value};

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
    let orders = rpc.call_database(
        api_id,
        "get_limit_orders",
        json!([base_asset_id, quote_asset_id, 100]),
    )?;
    let Some(orders) = orders.as_array() else {
        return Err("get_limit_orders result is not an array".into());
    };
    for order in orders {
        if order.get("seller").and_then(Value::as_str) == Some(seller_id) {
            if let Some(id) = order.get("id").and_then(Value::as_str) {
                return Ok(Some(id.to_string()));
            }
        }
    }
    Ok(None)
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
