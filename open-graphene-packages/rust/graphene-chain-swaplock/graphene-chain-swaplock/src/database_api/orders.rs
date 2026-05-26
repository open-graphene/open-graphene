use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::LimitOrderObject;
use open_graphene_sdk_core::{AccountIdRef, AssetIdRef, LimitOrderIdRef};

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
    let orders_json = rpc.get_limit_orders(
        api_id,
        base_asset_id.to_string(),
        quote_asset_id.to_string(),
        100,
    )?;
    let orders: Vec<LimitOrderObject> = serde_json::from_value(orders_json)?;

    find_order_id_for_seller(orders, &seller_id)
}

fn find_order_id_for_seller(
    orders: Vec<LimitOrderObject>,
    seller_id: &AccountIdRef,
) -> Result<Option<String>, Box<dyn Error>> {
    for order in orders {
        let order_seller = AccountIdRef::parse(&order.seller.0)?;
        if &order_seller == seller_id {
            return Ok(Some(LimitOrderIdRef::parse(&order.id.0)?.to_string()));
        }
    }

    Ok(None)
}

pub fn wait_for_order_gone(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    order_id: &str,
) -> Result<(), Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    let parsed_order_id = LimitOrderIdRef::parse(order_id)?;
    for _ in 0..20 {
        if !open_graphene_sdk_live::limit_order_exists(rpc.session_mut(), &parsed_order_id)? {
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

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_chain_swaplock_bindings::generated::{AccountId, Asset, AssetId, LimitOrderId, Price};

    #[test]
    fn find_order_id_for_seller_returns_matching_generated_order_id() {
        let orders = vec![limit_order("1.7.14", "1.2.99"), limit_order("1.7.15", "1.2.100")];
        let seller = AccountIdRef::parse("1.2.100").expect("valid seller");

        let found = find_order_id_for_seller(orders, &seller).expect("search orders");

        assert_eq!(found.as_deref(), Some("1.7.15"));
    }

    #[test]
    fn find_order_id_for_seller_validates_generated_order_ids() {
        let orders = vec![limit_order("1.2.15", "1.2.100")];
        let seller = AccountIdRef::parse("1.2.100").expect("valid seller");

        let err = find_order_id_for_seller(orders, &seller).expect_err("invalid order id");

        assert!(
            err.to_string().contains("expected object id space 1, type 7"),
            "unexpected error: {err}"
        );
    }

    fn limit_order(id: &str, seller: &str) -> LimitOrderObject {
        LimitOrderObject {
            id: LimitOrderId(id.to_string()),
            expiration: "2026-05-26T12:00:00".to_string(),
            seller: AccountId(seller.to_string()),
            for_sale: 100_000,
            sell_price: Price {
                base: Asset {
                    amount: 100_000,
                    asset_id: AssetId("1.3.0".to_string()),
                },
                quote: Asset {
                    amount: 50_000,
                    asset_id: AssetId("1.3.1".to_string()),
                },
            },
            filled_amount: "0".to_string(),
            deferred_fee: 0,
            deferred_paid_fee: Asset {
                amount: 0,
                asset_id: AssetId("1.3.0".to_string()),
            },
            is_settled_debt: false,
            on_fill: vec![],
            take_profit_order_id: None,
        }
    }
}
