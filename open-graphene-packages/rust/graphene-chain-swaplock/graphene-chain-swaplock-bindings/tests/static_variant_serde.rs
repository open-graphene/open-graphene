use graphene_chain_swaplock_bindings::generated::{
    AccountId, Asset, AssetId, FutureExtensions, Operation, TransferOperation,
};

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
        extensions: FutureExtensions::VoidT(Box::new(())),
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
            "extensions": [0, null]
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
