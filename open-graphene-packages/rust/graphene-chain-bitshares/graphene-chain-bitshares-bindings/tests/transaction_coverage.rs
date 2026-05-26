use graphene_chain_bitshares_bindings::generated::FcSerialize;
use graphene_chain_bitshares_bindings::generated::ids::{AccountId, AssetId, LimitOrderId};
use graphene_chain_bitshares_bindings::generated::operations::TransferOperation;
use graphene_chain_bitshares_bindings::generated::static_variants::{FutureExtensions, Operation};
use graphene_chain_bitshares_bindings::generated::types::{
    Asset, LimitOrderObject, Signature, SignedTransaction, Transaction,
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
        extensions: FutureExtensions::VoidT(Box::new(())),
    }))
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
fn bitshares_transaction_serializes_transfer_operation_vector() {
    let transaction = Transaction {
        ref_block_num: 2,
        ref_block_prefix: 3,
        expiration: "2026-05-25T12:01:00".to_string(),
        operations: vec![transfer_operation()],
        extensions: FutureExtensions::VoidT(Box::new(())),
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
        extensions: FutureExtensions::VoidT(Box::new(())),
        signatures: vec![Signature(vec![0x1f; 65])],
    };

    let bytes = signed_transaction.to_fc_bytes().unwrap();

    assert!(bytes.ends_with(&[0x1f; 65]));
}
