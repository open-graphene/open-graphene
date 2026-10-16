use graphene_chain_rsquared_bindings::generated::{
    FcSerialize, FeeParameters, HtlcObject, Operation,
};
use serde_json::json;

#[test]
fn personal_data_uses_rsquared_tag_and_reflected_field_order() {
    let value = json!([39, {
        "fee": { "amount": 0, "asset_id": "1.3.0" },
        "subject_account": "1.2.1",
        "operator_account": "1.2.2",
        "url": "a",
        "hash": "b",
        "storage_data": "c"
    }]);
    let operation: Operation = serde_json::from_value(value.clone()).unwrap();
    let mut bytes = Vec::new();
    operation.fc_serialize(&mut bytes).unwrap();

    assert_eq!(
        bytes,
        [39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 1, 97, 1, 98, 1, 99]
    );
    assert_eq!(serde_json::to_value(operation).unwrap(), value);
}

#[test]
fn legacy_fee_parameters_preserve_wide_integer_values() {
    let value = json!([39, { "fee": "9007199254740993", "price_per_kbyte": 10 }]);
    let fee: FeeParameters = serde_json::from_value(value).unwrap();
    let FeeParameters::PersonalDataCreateOperationFeeParametersType(parameters) = fee else {
        panic!("wrong fee parameter arm");
    };

    assert_eq!(parameters.fee, 9007199254740993);
    assert_eq!(parameters.price_per_kbyte, 10);
    assert!(serde_json::from_value::<Operation>(json!([64, {}])).is_err());
}

#[test]
fn htlc_preserves_inline_nested_transfer_and_conditions() {
    let value = json!({
        "id": "1.21.1",
        "transfer": { "from": "1.2.1", "to": "1.2.2", "amount": 10, "asset_id": "1.3.0" },
        "conditions": {
            "hash_lock": { "preimage_hash": [2, "00".repeat(32)], "preimage_size": 32 },
            "time_lock": { "expiration": "2026-10-16T12:00:00" }
        }
    });
    let htlc: HtlcObject = serde_json::from_value(value.clone()).unwrap();

    assert_eq!(htlc.transfer.amount, 10);
    assert_eq!(htlc.conditions.hash_lock.preimage_size, 32);
    assert_eq!(serde_json::to_value(htlc).unwrap(), value);
}
