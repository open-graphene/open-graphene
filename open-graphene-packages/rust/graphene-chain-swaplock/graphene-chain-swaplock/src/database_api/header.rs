use std::error::Error;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::DynamicGlobalPropertyObject;
use open_graphene_sdk_core::HeadBlock;

use crate::rpc::GrapheneRpc;

pub fn head_block(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
) -> Result<HeadBlock, Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != database_api_id {
        return Err(format!(
            "database API id mismatch: expected {expected}, got {database_api_id}"
        )
        .into());
    }

    let properties: DynamicGlobalPropertyObject =
        serde_json::from_value(rpc.get_dynamic_global_properties(database_api_id)?)?;
    Ok(head_block_from_dynamic_global_properties(properties))
}

pub fn next_transaction_header(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    expiration: Duration,
) -> Result<open_graphene_sdk_core::TransactionHeader, Box<dyn Error>> {
    open_graphene_sdk_core::transaction_header_from_head(
        &head_block(rpc, database_api_id)?,
        expiration,
    )
    .map_err(Into::into)
}

fn head_block_from_dynamic_global_properties(properties: DynamicGlobalPropertyObject) -> HeadBlock {
    HeadBlock {
        number: u64::from(properties.head_block_number),
        id: bytes_to_lower_hex(&properties.head_block_id),
        time: properties.time,
    }
}

fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

#[cfg(test)]
mod tests {
    use graphene_chain_swaplock_bindings::generated::{DynamicGlobalPropertyId, WitnessId};

    use super::*;

    #[test]
    fn parses_head_block_from_generated_dynamic_global_properties() {
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
            "total_pob": "0",
            "total_inactive": "0",
            "accounts_registered_this_interval": 0,
            "recently_missed_count": 0,
            "current_aslot": 123456,
            "recent_slots_filled": "340282366920938463463374607431768211455",
            "dynamic_flags": 0,
            "last_irreversible_block_num": 609700
        });

        let properties: DynamicGlobalPropertyObject =
            serde_json::from_value(json).expect("deserialize dynamic global properties");
        let head = head_block_from_dynamic_global_properties(properties);

        assert_eq!(
            head,
            HeadBlock {
                number: 609_782,
                id: "00094df644fe617490ae116a0100400d03000000".to_string(),
                time: "2026-05-25T12:00:00".to_string(),
            }
        );
    }

    #[test]
    fn dynamic_global_properties_rejects_wrong_block_id_length() {
        let json = serde_json::json!({
            "id": "2.1.0",
            "head_block_number": 609782,
            "head_block_id": "00094df6",
            "time": "2026-05-25T12:00:00",
            "current_witness": "1.6.1",
            "next_maintenance_time": "2026-05-25T13:00:00",
            "last_vote_tally_time": "2026-05-25T11:00:00",
            "last_budget_time": "2026-05-25T11:00:00",
            "witness_budget": 0,
            "total_pob": 0,
            "total_inactive": 0,
            "accounts_registered_this_interval": 0,
            "recently_missed_count": 0,
            "current_aslot": 123456,
            "recent_slots_filled": "0",
            "dynamic_flags": 0,
            "last_irreversible_block_num": 609700
        });

        let error = serde_json::from_value::<DynamicGlobalPropertyObject>(json).unwrap_err();

        assert!(error.to_string().contains("expected 20 fixed bytes"));
    }

    #[test]
    fn builds_head_block_from_generated_dynamic_global_properties_value() {
        let properties = DynamicGlobalPropertyObject {
            id: DynamicGlobalPropertyId("2.1.0".to_string()),
            head_block_number: 609_782,
            head_block_id: vec![
                0x00, 0x09, 0x4d, 0xf6, 0x44, 0xfe, 0x61, 0x74, 0x90, 0xae, 0x11, 0x6a, 0x01, 0x00,
                0x40, 0x0d, 0x03, 0x00, 0x00, 0x00,
            ],
            time: "2026-05-25T12:00:00".to_string(),
            current_witness: WitnessId("1.6.1".to_string()),
            next_maintenance_time: "2026-05-25T13:00:00".to_string(),
            last_vote_tally_time: "2026-05-25T11:00:00".to_string(),
            last_budget_time: "2026-05-25T11:00:00".to_string(),
            witness_budget: 0,
            total_pob: 0,
            total_inactive: 0,
            accounts_registered_this_interval: 0,
            recently_missed_count: 0,
            current_aslot: 123_456,
            recent_slots_filled: "0".to_string(),
            dynamic_flags: 0,
            last_irreversible_block_num: 609_700,
        };

        let head = head_block_from_dynamic_global_properties(properties);

        assert_eq!(head.number, 609_782);
        assert_eq!(head.id, "00094df644fe617490ae116a0100400d03000000");
        assert_eq!(head.time, "2026-05-25T12:00:00");
    }
}
