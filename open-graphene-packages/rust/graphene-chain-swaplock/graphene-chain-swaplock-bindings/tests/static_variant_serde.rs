use graphene_chain_swaplock_bindings::generated::rpc::ProtocolObject;
use graphene_chain_swaplock_bindings::generated::{
    AccountId, Asset, AssetId, LimitOrderId, LimitOrderObject, Operation, TransferOperation,
};

#[test]
fn protocol_object_deserializes_by_object_id_type() {
    let known = serde_json::json!({
        "id": "2.11.0",
        "chain_id": "9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9",
        "immutable_parameters": {
            "min_committee_member_count": 1,
            "min_witness_count": 1,
            "num_special_accounts": 0,
            "num_special_assets": 0
        }
    });

    let object: ProtocolObject = serde_json::from_value(known).expect("known protocol object");
    assert!(matches!(object, ProtocolObject::ChainProperty(_)));

    let unknown_json = serde_json::json!({"id": "1.5.0", "url": "https://example.invalid"});
    let object: ProtocolObject =
        serde_json::from_value(unknown_json).expect("unknown protocol object");
    assert!(matches!(object, ProtocolObject::Unknown(_)));
}

#[test]
fn operation_serializes_as_graphene_tagged_tuple() {
    let operation = Operation::TransferOperation(Box::new(TransferOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        from: AccountId("1.2.1".to_string()),
        to: AccountId("1.2.2".to_string()),
        amount: Asset {
            amount: 100_000,
            asset_id: AssetId("1.3.0".to_string()),
        },
        memo: None,
        extensions: vec![],
    }));

    let json = serde_json::to_value(&operation).expect("serialize operation");

    assert_eq!(json[0], 0);
    assert_eq!(json[1]["from"], "1.2.1");
    assert_eq!(json[1]["to"], "1.2.2");
    assert_eq!(json[1]["amount"]["amount"], 100_000);
}

#[test]
fn operation_deserializes_from_graphene_tagged_tuple() {
    let json = serde_json::json!([
        0,
        {
            "fee": { "amount": 0, "asset_id": "1.3.0" },
            "from": "1.2.1",
            "to": "1.2.2",
            "amount": { "amount": 100000, "asset_id": "1.3.0" },
            "memo": null,
            "extensions": []
        }
    ]);

    let operation: Operation = serde_json::from_value(json).expect("deserialize operation");

    match operation {
        Operation::TransferOperation(transfer) => {
            assert_eq!(transfer.from.0, "1.2.1");
            assert_eq!(transfer.to.0, "1.2.2");
            assert_eq!(transfer.amount.amount, 100_000);
        }
        other => panic!("unexpected operation variant: {other:?}"),
    }
}

#[test]
fn limit_order_object_deserializes_from_graphene_json() {
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
