use serde_json::{json, Value};

pub struct SignedTransactionJsonParts<'a> {
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub expiration: &'a str,
    pub operations: Vec<Value>,
    pub signatures: Vec<&'a [u8]>,
}

pub fn signed_transaction_broadcast_json(parts: SignedTransactionJsonParts<'_>) -> Value {
    json!({
        "ref_block_num": parts.ref_block_num,
        "ref_block_prefix": parts.ref_block_prefix,
        "expiration": parts.expiration,
        "operations": parts.operations,
        "extensions": [],
        "signatures": parts
            .signatures
            .iter()
            .map(|signature| hex(signature))
            .collect::<Vec<_>>(),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_signed_transaction_broadcast_envelope() {
        let value = signed_transaction_broadcast_json(SignedTransactionJsonParts {
            ref_block_num: 42,
            ref_block_prefix: 7,
            expiration: "2026-05-25T19:00:00",
            operations: vec![json!([0, { "from": "1.2.3" }])],
            signatures: vec![&[0, 1, 15, 16, 255], &[171, 205]],
        });

        assert_eq!(
            value,
            json!({
                "ref_block_num": 42,
                "ref_block_prefix": 7,
                "expiration": "2026-05-25T19:00:00",
                "operations": [[0, { "from": "1.2.3" }]],
                "extensions": [],
                "signatures": ["00010f10ff", "abcd"],
            })
        );
    }
}
