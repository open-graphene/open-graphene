use graphene_chain_bitshares_bindings::generated::FcSerialize;
use graphene_chain_bitshares_bindings::generated::ids::{
    AccountId, AssetId, DynamicGlobalPropertyId, LimitOrderId, WitnessId,
};
use graphene_chain_bitshares_bindings::generated::operations::TransferOperation;
use graphene_chain_bitshares_bindings::generated::static_variants::Operation;
use graphene_chain_bitshares_bindings::generated::types::{
    Asset, AssetObject, DynamicGlobalPropertyObject, LimitOrderObject, Signature,
    SignedTransaction, Transaction,
};

fn transfer_operation() -> Operation {
    Operation::TransferOperation(Box::new(TransferOperation {
        fee: Asset {
            amount: 200_000,
            asset_id: AssetId("1.3.0".to_string()),
        },
        from: AccountId("1.2.100".to_string()),
        to: AccountId("1.2.0".to_string()),
        amount: Asset {
            amount: 100_000,
            asset_id: AssetId("1.3.0".to_string()),
        },
        memo: None,
        extensions: vec![],
    }))
}

#[test]
fn bitshares_account_balance_asset_deserializes_from_graphene_json() {
    let json = serde_json::json!({
        "amount": "12345",
        "asset_id": "1.3.0"
    });

    let balance: Asset = serde_json::from_value(json).expect("deserialize balance asset");

    assert_eq!(balance.amount, 12_345);
    assert_eq!(balance.asset_id, AssetId("1.3.0".to_string()));
}

#[test]
fn bitshares_asset_object_deserializes_from_get_objects_json() {
    let json = serde_json::json!({
        "id": "1.3.0",
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
    });

    let asset: AssetObject = serde_json::from_value(json).expect("deserialize asset object");

    assert_eq!(asset.id, AssetId("1.3.0".to_string()));
    assert_eq!(asset.symbol, "BTS");
    assert_eq!(asset.precision, 5);
    assert_eq!(asset.issuer, AccountId("1.2.0".to_string()));
    assert_eq!(asset.options.max_supply, 1_000_000_000_000_000);
    assert_eq!(asset.dynamic_asset_data_id.0, "2.3.0");
    assert_eq!(asset.bitasset_data_id, None);
    assert_eq!(asset.buyback_account, None);
    assert_eq!(asset.for_liquidity_pool, None);
}

#[test]
fn bitshares_limit_order_object_deserializes_from_graphene_json() {
    let json = serde_json::json!({
        "id": "1.7.14",
        "expiration": "2026-05-26T12:00:00",
        "seller": "1.2.100",
        "for_sale": "100000",
        "sell_price": {
            "base": { "amount": 100000, "asset_id": "1.3.0" },
            "quote": { "amount": 50000, "asset_id": "1.3.1" }
        },
        "filled_amount": "0",
        "deferred_fee": 0,
        "deferred_paid_fee": { "amount": 0, "asset_id": "1.3.0" },
        "is_settled_debt": false,
        "on_fill": [],
        "take_profit_order_id": null
    });

    let order: LimitOrderObject = serde_json::from_value(json).expect("deserialize order");

    assert_eq!(order.id, LimitOrderId("1.7.14".to_string()));
    assert_eq!(order.seller, AccountId("1.2.100".to_string()));
    assert_eq!(order.filled_amount, "0");
    assert_eq!(order.for_sale, 100_000);
    assert_eq!(order.sell_price.base.asset_id, AssetId("1.3.0".to_string()));
    assert_eq!(order.on_fill.len(), 0);
    assert_eq!(order.take_profit_order_id, None);
}

#[test]
fn bitshares_dynamic_global_property_object_deserializes_from_graphene_json() {
    let json = serde_json::json!({
        "id": "2.1.0",
        "head_block_number": 609782,
        "head_block_id": "00094df644fe617490ae116a0100400d03000000",
        "time": "2026-05-25T12:00:00",
        "current_witness": "1.6.1",
        "next_maintenance_time": "2026-05-25T13:00:00",
        "last_vote_tally_time": "2026-05-25T11:00:00",
        "last_budget_time": "2026-05-25T11:00:00",
        "witness_budget": "0",
        "total_pob": "1",
        "total_inactive": "2",
        "accounts_registered_this_interval": 3,
        "recently_missed_count": 4,
        "current_aslot": 123456,
        "recent_slots_filled": "340282366920938463463374607431768211455",
        "dynamic_flags": 0,
        "last_irreversible_block_num": 609700
    });

    let properties: DynamicGlobalPropertyObject =
        serde_json::from_value(json).expect("deserialize dynamic global properties");

    assert_eq!(properties.id, DynamicGlobalPropertyId("2.1.0".to_string()));
    assert_eq!(properties.head_block_number, 609_782);
    assert_eq!(
        properties.head_block_id,
        vec![
            0x00, 0x09, 0x4d, 0xf6, 0x44, 0xfe, 0x61, 0x74, 0x90, 0xae, 0x11, 0x6a, 0x01, 0x00,
            0x40, 0x0d, 0x03, 0x00, 0x00, 0x00,
        ]
    );
    assert_eq!(properties.time, "2026-05-25T12:00:00");
    assert_eq!(properties.current_witness, WitnessId("1.6.1".to_string()));
    assert_eq!(properties.witness_budget, 0);
    assert_eq!(properties.total_pob, 1);
    assert_eq!(properties.total_inactive, 2);
    assert_eq!(
        properties.recent_slots_filled,
        "340282366920938463463374607431768211455"
    );
}

#[test]
fn bitshares_transaction_serializes_transfer_operation_vector() {
    let transaction = Transaction {
        ref_block_num: 2,
        ref_block_prefix: 3,
        expiration: "2026-05-25T12:01:00".to_string(),
        operations: vec![transfer_operation()],
        extensions: vec![],
    };

    let bytes = transaction.to_fc_bytes().unwrap();

    assert_eq!(&bytes[0..2], &2u16.to_le_bytes());
    assert_eq!(&bytes[2..6], &3u32.to_le_bytes());
    assert!(bytes.len() > 6);
}

#[test]
fn bitshares_signed_transaction_serializes_signature_vector() {
    let signed_transaction = SignedTransaction {
        ref_block_num: 2,
        ref_block_prefix: 3,
        expiration: "2026-05-25T12:01:00".to_string(),
        operations: vec![transfer_operation()],
        extensions: vec![],
        signatures: vec![Signature(vec![0x1f; 65])],
    };

    let bytes = signed_transaction.to_fc_bytes().unwrap();

    assert!(bytes.ends_with(&[0x1f; 65]));
}
