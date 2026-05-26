use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::{AssetDynamicDataObject, AssetObject};
use open_graphene_sdk_core::{AssetIdRef, ObjectId};

use crate::database_api::objects::typed_object_from_get_objects_result;
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

pub fn asset_dynamic_data_object(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    asset_dynamic_data_id: &str,
) -> Result<Option<AssetDynamicDataObject>, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    let requested_asset_dynamic_data_id = parse_asset_dynamic_data_id(asset_dynamic_data_id)?;
    let result = rpc.get_objects(api_id, [requested_asset_dynamic_data_id.to_string()])?;
    asset_dynamic_data_object_from_get_objects_result(&requested_asset_dynamic_data_id, result)
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
    value: serde_json::Value,
) -> Result<Option<AssetObject>, Box<dyn Error>> {
    typed_object_from_get_objects_result(
        "asset",
        requested_asset_id,
        value,
        AssetIdRef::parse,
        |asset: &AssetObject| asset.id.0.as_str(),
    )
}

fn asset_dynamic_data_object_from_get_objects_result(
    requested_asset_dynamic_data_id: &ObjectId,
    value: serde_json::Value,
) -> Result<Option<AssetDynamicDataObject>, Box<dyn Error>> {
    typed_object_from_get_objects_result(
        "asset dynamic data",
        requested_asset_dynamic_data_id,
        value,
        parse_asset_dynamic_data_id,
        |dynamic_data: &AssetDynamicDataObject| dynamic_data.id.0.as_str(),
    )
}

fn parse_asset_dynamic_data_id(
    value: &str,
) -> Result<ObjectId, open_graphene_sdk_core::ObjectIdError> {
    ObjectId::parse(value)?.require_type(2, 3)
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
    use graphene_chain_swaplock_bindings::generated::{
        AccountId, AssetDynamicDataId, AssetDynamicDataObject, AssetId, AssetObject,
    };

    use super::*;

    fn asset_object_json(id: &str) -> serde_json::Value {
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

    fn asset_dynamic_data_object_json(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "current_supply": "1000000000000",
            "confidential_supply": 0,
            "accumulated_fees": "1000",
            "accumulated_collateral_fees": 0,
            "fee_pool": "200000"
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

        assert!(
            error
                .to_string()
                .contains("get_objects asset id mismatch: expected 1.3.0, got 1.3.1")
        );
    }

    #[test]
    fn generated_asset_object_rejects_wrong_get_objects_slot_count() {
        let requested_asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let result = serde_json::json!([asset_object_json("1.3.0"), null]);

        let error = asset_object_from_get_objects_result(&requested_asset_id, result).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("get_objects asset response must contain exactly one slot, got 2")
        );
    }

    #[test]
    fn generated_asset_object_can_be_constructed_as_generated_type() {
        let asset: AssetObject = serde_json::from_value(asset_object_json("1.3.0")).unwrap();

        assert_eq!(asset.id, AssetId("1.3.0".to_string()));
    }

    #[test]
    fn generated_asset_dynamic_data_object_deserializes_from_get_objects_slot() {
        let requested_dynamic_data_id = parse_asset_dynamic_data_id("2.3.0").unwrap();
        let result = serde_json::json!([asset_dynamic_data_object_json("2.3.0")]);

        let dynamic_data =
            asset_dynamic_data_object_from_get_objects_result(&requested_dynamic_data_id, result)
                .expect("parse asset dynamic data object")
                .expect("asset dynamic data should exist");

        assert_eq!(dynamic_data.id, AssetDynamicDataId("2.3.0".to_string()));
        assert_eq!(dynamic_data.current_supply, 1_000_000_000_000);
        assert_eq!(dynamic_data.confidential_supply, 0);
        assert_eq!(dynamic_data.accumulated_fees, 1_000);
        assert_eq!(dynamic_data.fee_pool, 200_000);
    }

    #[test]
    fn generated_asset_dynamic_data_object_preserves_missing_get_objects_slot() {
        let requested_dynamic_data_id = parse_asset_dynamic_data_id("2.3.999").unwrap();
        let result = serde_json::json!([null]);

        let dynamic_data =
            asset_dynamic_data_object_from_get_objects_result(&requested_dynamic_data_id, result)
                .expect("parse missing asset dynamic data slot");

        assert!(dynamic_data.is_none());
    }

    #[test]
    fn generated_asset_dynamic_data_object_rejects_mismatched_get_objects_id() {
        let requested_dynamic_data_id = parse_asset_dynamic_data_id("2.3.0").unwrap();
        let result = serde_json::json!([asset_dynamic_data_object_json("2.3.1")]);

        let error =
            asset_dynamic_data_object_from_get_objects_result(&requested_dynamic_data_id, result)
                .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("get_objects asset dynamic data id mismatch: expected 2.3.0, got 2.3.1")
        );
    }

    #[test]
    fn generated_asset_dynamic_data_object_rejects_wrong_get_objects_slot_count() {
        let requested_dynamic_data_id = parse_asset_dynamic_data_id("2.3.0").unwrap();
        let result = serde_json::json!([asset_dynamic_data_object_json("2.3.0"), null]);

        let error =
            asset_dynamic_data_object_from_get_objects_result(&requested_dynamic_data_id, result)
                .unwrap_err();

        assert!(error.to_string().contains(
            "get_objects asset dynamic data response must contain exactly one slot, got 2"
        ));
    }

    #[test]
    fn generated_asset_dynamic_data_object_can_be_constructed_as_generated_type() {
        let dynamic_data: AssetDynamicDataObject =
            serde_json::from_value(asset_dynamic_data_object_json("2.3.0")).unwrap();

        assert_eq!(dynamic_data.id, AssetDynamicDataId("2.3.0".to_string()));
    }
}
