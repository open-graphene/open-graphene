use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::AssetObject;
use open_graphene_sdk_core::AssetIdRef;
use serde_json::Value;

use crate::rpc::GrapheneRpc;

pub fn lookup_asset_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<String, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    Ok(open_graphene_sdk_live::lookup_asset_id(rpc.session_mut(), symbol)?.to_string())
}

pub fn lookup_asset_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    Ok(
        open_graphene_sdk_live::lookup_asset_id_optional(rpc.session_mut(), symbol)?
            .map(|asset_id| asset_id.to_string()),
    )
}

pub fn asset_object(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    asset_id: &str,
) -> Result<Option<AssetObject>, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    let requested_asset_id = AssetIdRef::parse(asset_id)?;
    let result = rpc.get_objects(api_id, [requested_asset_id.to_string()])?;
    asset_object_from_get_objects_result(&requested_asset_id, result)
}

pub fn wait_for_asset(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    for _ in 0..20 {
        if let Some(asset_id) = lookup_asset_id_optional(rpc, api_id, symbol)? {
            return Ok(Some(asset_id));
        }
        sleep(Duration::from_millis(500));
    }
    Ok(None)
}

fn asset_object_from_get_objects_result(
    requested_asset_id: &AssetIdRef,
    value: Value,
) -> Result<Option<AssetObject>, Box<dyn Error>> {
    let values = value
        .as_array()
        .ok_or("get_objects asset response must be an array")?;
    if values.len() != 1 {
        return Err(format!(
            "get_objects asset response must contain exactly one slot, got {}",
            values.len()
        )
        .into());
    }

    let Some(slot) = values.first() else {
        return Err("get_objects asset response must contain one slot".into());
    };
    if slot.is_null() {
        return Ok(None);
    }

    let asset: AssetObject = serde_json::from_value(slot.clone())?;
    let returned_asset_id = AssetIdRef::parse(&asset.id.0)?;
    if &returned_asset_id != requested_asset_id {
        return Err(format!(
            "get_objects asset id mismatch: expected {requested_asset_id}, got {returned_asset_id}"
        )
        .into());
    }

    Ok(Some(asset))
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
    use graphene_chain_swaplock_bindings::generated::{AccountId, AssetId, AssetObject};

    use super::*;

    fn asset_object_json(id: &str) -> Value {
        serde_json::json!({
            "id": id,
            "symbol": "BTS",
            "precision": 5,
            "issuer": "1.2.0",
            "options": {
                "max_supply": "1000000000000000",
                "market_fee_percent": 0,
                "max_market_fee": "0",
                "issuer_permissions": 79,
                "flags": 0,
                "core_exchange_rate": {
                    "base": { "amount": 1, "asset_id": "1.3.0" },
                    "quote": { "amount": 1, "asset_id": "1.3.0" }
                },
                "whitelist_authorities": [],
                "blacklist_authorities": [],
                "whitelist_markets": [],
                "blacklist_markets": [],
                "description": "core asset",
                "extensions": {}
            },
            "dynamic_asset_data_id": "2.3.0",
            "bitasset_data_id": null,
            "buyback_account": null,
            "for_liquidity_pool": null,
            "creation_block_num": 1,
            "creation_time": "2026-05-26T12:00:00"
        })
    }

    #[test]
    fn generated_asset_object_deserializes_from_get_objects_slot() {
        let requested_asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let result = serde_json::json!([asset_object_json("1.3.0")]);

        let asset = asset_object_from_get_objects_result(&requested_asset_id, result)
            .expect("parse asset object")
            .expect("asset should exist");

        assert_eq!(asset.id, AssetId("1.3.0".to_string()));
        assert_eq!(asset.symbol, "BTS");
        assert_eq!(asset.precision, 5);
        assert_eq!(asset.issuer, AccountId("1.2.0".to_string()));
        assert_eq!(asset.options.max_supply, 1_000_000_000_000_000);
    }

    #[test]
    fn generated_asset_object_preserves_missing_get_objects_slot() {
        let requested_asset_id = AssetIdRef::parse("1.3.999").unwrap();
        let result = serde_json::json!([null]);

        let asset = asset_object_from_get_objects_result(&requested_asset_id, result)
            .expect("parse missing asset slot");

        assert!(asset.is_none());
    }

    #[test]
    fn generated_asset_object_rejects_mismatched_get_objects_id() {
        let requested_asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let result = serde_json::json!([asset_object_json("1.3.1")]);

        let error = asset_object_from_get_objects_result(&requested_asset_id, result).unwrap_err();

        assert!(error
            .to_string()
            .contains("get_objects asset id mismatch: expected 1.3.0, got 1.3.1"));
    }

    #[test]
    fn generated_asset_object_rejects_wrong_get_objects_slot_count() {
        let requested_asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let result = serde_json::json!([asset_object_json("1.3.0"), null]);

        let error = asset_object_from_get_objects_result(&requested_asset_id, result).unwrap_err();

        assert!(error
            .to_string()
            .contains("get_objects asset response must contain exactly one slot, got 2"));
    }

    #[test]
    fn generated_asset_object_can_be_constructed_as_generated_type() {
        let asset: AssetObject = serde_json::from_value(asset_object_json("1.3.0")).unwrap();

        assert_eq!(asset.id, AssetId("1.3.0".to_string()));
    }
}
