use graphene_chain_swaplock_bindings::generated::{
    AccountCreateOperation, AccountCreateOperationExt, AccountId, AccountOptions, Asset, AssetId,
    Authority, FcSerialize, FcSerializeError, FutureExtensions, LimitOrderCancelOperation,
    LimitOrderCreateOperation, LimitOrderId, Operation, TransferOperation,
};

fn sample_transfer_operation() -> TransferOperation {
    TransferOperation {
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
    }
}

fn sample_limit_order_create_operation() -> LimitOrderCreateOperation {
    LimitOrderCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        seller: AccountId("1.2.1".to_string()),
        amount_to_sell: Asset {
            amount: 1,
            asset_id: AssetId("1.3.0".to_string()),
        },
        min_to_receive: Asset {
            amount: 2,
            asset_id: AssetId("1.3.0".to_string()),
        },
        fill_or_kill: true,
        extensions: FutureExtensions::VoidT(Box::new(())),
    }
}

fn sample_limit_order_cancel_operation() -> LimitOrderCancelOperation {
    LimitOrderCancelOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        fee_paying_account: AccountId("1.2.1".to_string()),
        order: LimitOrderId("1.7.1".to_string()),
        extensions: FutureExtensions::VoidT(Box::new(())),
    }
}

fn expected_limit_order_create_payload() -> Vec<u8> {
    vec![
        // fee: amount 0 + asset instance 0
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        // seller account instance 1
        1,
        // amount_to_sell: amount 1 + asset instance 0
        1, 0, 0, 0, 0, 0, 0, 0, 0,
        // min_to_receive: amount 2 + asset instance 0
        2, 0, 0, 0, 0, 0, 0, 0, 0,
        // fill_or_kill true
        1,
        // extensions: future_extensions VoidT static variant tag 0
        0,
    ]
}

fn expected_limit_order_cancel_payload() -> Vec<u8> {
    vec![
        // fee: amount 0 + asset instance 0
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        // fee_paying_account account instance 1
        1,
        // order limit_order instance 1
        1,
        // extensions: future_extensions VoidT static variant tag 0
        0,
    ]
}

#[test]
fn asset_fc_serializes_amount_and_asset_instance() {
    let asset = Asset {
        amount: 100_000,
        asset_id: AssetId("1.3.0".to_string()),
    };

    assert_eq!(
        asset.to_fc_bytes().expect("serialize asset"),
        vec![0xa0, 0x86, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn transfer_operation_fc_rejects_invalid_account_id() {
    let mut transfer = sample_transfer_operation();
    transfer.from = AccountId("bad".to_string());

    let err = transfer.to_fc_bytes().expect_err("invalid account id fails");

    assert!(matches!(
        err,
        FcSerializeError::InvalidProtocolObjectId {
            expected_space: Some(1),
            expected_type: Some(2),
            ..
        }
    ));
}

#[test]
fn operation_fc_serializes_transfer_tag_and_payload() {
    let operation = Operation::TransferOperation(Box::new(sample_transfer_operation()));
    let bytes = operation.to_fc_bytes().expect("serialize operation");

    assert_eq!(bytes[0], 0);
    assert_eq!(bytes[1..9], [0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(bytes[9], 0); // fee asset instance 0
    assert_eq!(bytes[10], 1); // from account instance 1
    assert_eq!(bytes[11], 2); // to account instance 2
}

#[test]
fn limit_order_create_operation_fc_serializes_known_fields() {
    let operation = sample_limit_order_create_operation();

    assert_eq!(
        operation.to_fc_bytes().expect("serialize limit order create"),
        expected_limit_order_create_payload()
    );
}

#[test]
fn operation_fc_serializes_limit_order_create_tag_and_payload() {
    let operation = Operation::LimitOrderCreateOperation(Box::new(sample_limit_order_create_operation()));
    let bytes = operation.to_fc_bytes().expect("serialize operation");

    let mut expected = vec![1];
    expected.extend(expected_limit_order_create_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn limit_order_cancel_operation_fc_serializes_known_fields() {
    let operation = sample_limit_order_cancel_operation();

    assert_eq!(
        operation.to_fc_bytes().expect("serialize limit order cancel"),
        expected_limit_order_cancel_payload()
    );
}

#[test]
fn operation_fc_serializes_limit_order_cancel_tag_and_payload() {
    let operation = Operation::LimitOrderCancelOperation(Box::new(sample_limit_order_cancel_operation()));
    let bytes = operation.to_fc_bytes().expect("serialize operation");

    let mut expected = vec![2];
    expected.extend(expected_limit_order_cancel_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn operation_fc_reports_unsupported_variant() {
    let operation = Operation::AccountCreateOperation(Box::new(AccountCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        registrar: AccountId("1.2.1".to_string()),
        referrer: AccountId("1.2.2".to_string()),
        referrer_percent: 0,
        name: "alice".to_string(),
        owner: Authority {},
        active: Authority {},
        options: AccountOptions {
            memo_key: "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV".to_string(),
            voting_account: AccountId("1.2.5".to_string()),
            num_witness: 0,
            num_committee: 0,
            votes: Vec::new(),
            extensions: FutureExtensions::VoidT(Box::new(())),
        },
        extensions: AccountCreateOperationExt {
            null_ext: None,
            owner_special_authority: None,
            active_special_authority: None,
        },
    }));

    let err = operation.to_fc_bytes().expect_err("unsupported variant fails explicitly");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedVariant {
            variant: "AccountCreateOperation"
        }
    ));
}
