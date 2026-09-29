use graphene_chain_bitshares_bindings::generated::{AccountId, FcSerialize, ObjectId, Transaction};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../typescript/tests/fixtures/protocol-vectors.json"
    ))
    .unwrap()
}

#[test]
fn generic_object_id_uses_cpp_packed_u64_not_typed_instance_varint() {
    let fixture = vectors();
    for vector in fixture["objectIds"].as_array().unwrap() {
        let id = ObjectId::from(vector["id"].as_str().unwrap());
        let expected = decode_hex(vector["genericHex"].as_str().unwrap());
        assert_eq!(id.to_fc_bytes().unwrap(), expected);
    }
    assert_eq!(
        AccountId::from("1.2.345").to_fc_bytes().unwrap(),
        [0xd9, 0x02]
    );
    assert!(AccountId::from("1.3.345").to_fc_bytes().is_err());
}

#[test]
fn unsigned_transfer_matches_original_javascript_wire_vector() {
    let fixture = vectors();
    let tx: Transaction =
        serde_json::from_value(fixture["transfer"]["transaction"].clone()).unwrap();
    assert_eq!(
        tx.to_fc_bytes().unwrap(),
        decode_hex(fixture["transfer"]["hex"].as_str().unwrap())
    );
}

fn decode_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
