use graphene_swaplock_bindings::generated::{
    AccountId, Asset, AssetId, FcSerialize, FcSerializeError, FutureExtensions,
    LimitOrderCreateOperation, Operation, TransferOperation,
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
fn operation_fc_reports_unsupported_non_transfer_variant() {
    let operation = Operation::LimitOrderCreateOperation(Box::new(LimitOrderCreateOperation {
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
            amount: 1,
            asset_id: AssetId("1.3.0".to_string()),
        },
        fill_or_kill: false,
        extensions: FutureExtensions::VoidT(Box::new(())),
    }));

    let err = operation.to_fc_bytes().expect_err("non-transfer is explicit unsupported");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedVariant {
            variant: "LimitOrderCreateOperation"
        }
    ));
}
