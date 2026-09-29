//! Shared, public vectors also consumed by the native TypeScript runtime.
use open_graphene_fc::{FcSerialize, write_object_id, write_protocol_object_id};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../typescript/tests/fixtures/protocol-vectors.json"
    ))
    .unwrap()
}

#[test]
fn generic_and_typed_object_ids_have_distinct_wire_encodings() {
    for vector in vectors()["objectIds"].as_array().unwrap() {
        let id = vector["id"].as_str().unwrap();
        let mut generic = Vec::new();
        write_object_id(id, &mut generic).unwrap();
        assert_eq!(hex::encode(generic), vector["genericHex"], "{id}");
        let mut typed = Vec::new();
        write_protocol_object_id(id, None, None, &mut typed).unwrap();
        assert_eq!(hex::encode(typed), vector["typedHex"], "{id}");
    }
    for id in vectors()["invalidObjectIds"].as_array().unwrap() {
        let mut out = vec![42];
        assert!(write_object_id(id.as_str().unwrap(), &mut out).is_err());
        assert_eq!(out, [42], "invalid ID must not partially write");
        assert!(write_protocol_object_id(id.as_str().unwrap(), None, None, &mut out).is_err());
    }
}

#[test]
fn wide_integers_match_cross_language_vectors() {
    for vector in vectors()["integers"].as_array().unwrap() {
        let decimal = vector["decimal"].as_str().unwrap();
        let bytes = if vector["kind"] == "i64" {
            decimal.parse::<i64>().unwrap().to_fc_bytes().unwrap()
        } else {
            decimal.parse::<u64>().unwrap().to_fc_bytes().unwrap()
        };
        assert_eq!(hex::encode(bytes), vector["hex"]);
    }
}

#[test]
fn transfer_digest_matches_current_chain_and_wire_bytes() {
    let fixture = vectors();
    let mut preimage = hex::decode(fixture["transfer"]["chainId"].as_str().unwrap()).unwrap();
    preimage.extend(hex::decode(fixture["transfer"]["hex"].as_str().unwrap()).unwrap());
    assert_eq!(
        hex::encode(open_graphene_fc::sha256_bytes(&preimage)),
        fixture["transfer"]["digestHex"]
    );
}

#[cfg(feature = "signing")]
#[test]
fn public_key_fixture_signs_and_recovers_without_environment_secrets() {
    let fixture = vectors();
    let digest: [u8; 32] = hex::decode(fixture["signing"]["digestHex"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let signature = open_graphene_fc::sign_digest_compact(digest, [1; 32]).unwrap();
    assert_eq!(hex::encode(signature), fixture["signing"]["signatureHex"]);
    let recovered =
        open_graphene_fc::recover_public_key_from_compact_signature(digest, &signature).unwrap();
    assert_eq!(hex::encode(recovered), fixture["signing"]["publicKeyHex"]);
}
