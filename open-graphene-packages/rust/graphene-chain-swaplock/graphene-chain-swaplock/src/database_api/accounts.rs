use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::AccountObject;
use open_graphene_sdk_core::AccountIdRef;

use crate::database_api::objects::typed_object_from_get_objects_result;
use crate::rpc::GrapheneRpc;

pub fn lookup_account_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<String, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    Ok(open_graphene_sdk_live::lookup_account_id(rpc.session_mut(), account_name)?.to_string())
}

pub fn lookup_account_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    Ok(
        open_graphene_sdk_live::lookup_account_id_optional(rpc.session_mut(), account_name)?
            .map(|account_id| account_id.to_string()),
    )
}

pub fn account_object(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
) -> Result<Option<AccountObject>, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    let requested_account_id = AccountIdRef::parse(account_id)?;
    let result = rpc.get_objects(api_id, [requested_account_id.to_string()])?;
    account_object_from_get_objects_result(&requested_account_id, result)
}

pub fn wait_for_account(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    for _ in 0..20 {
        if let Some(account_id) = lookup_account_id_optional(rpc, api_id, account_name)? {
            return Ok(Some(account_id));
        }
        sleep(Duration::from_millis(500));
    }
    Ok(None)
}

pub fn active_public_key_for_account(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let Some(account) = account_object(rpc, api_id, account_id)? else {
        return Ok(None);
    };
    if account.active.key_auths.len() != 1 {
        return Ok(None);
    }
    Ok(account
        .active
        .key_auths
        .first()
        .map(|(public_key, _weight)| public_key.clone()))
}

fn account_object_from_get_objects_result(
    requested_account_id: &AccountIdRef,
    value: serde_json::Value,
) -> Result<Option<AccountObject>, Box<dyn Error>> {
    typed_object_from_get_objects_result(
        "account",
        requested_account_id,
        value,
        AccountIdRef::parse,
        |account: &AccountObject| account.id.0.as_str(),
    )
}

fn ensure_database_api_id(rpc: &mut GrapheneRpc, api_id: u64) -> Result<(), Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != api_id {
        return Err(format!("database API id mismatch: expected {expected}, got {api_id}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use graphene_chain_swaplock_bindings::generated::{AccountId, AccountObject};

    use super::*;

    fn account_object_json(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "membership_expiration_date": "1970-01-01T00:00:00",
            "registrar": "1.2.0",
            "referrer": "1.2.0",
            "lifetime_referrer": "1.2.0",
            "network_fee_percentage": 2000,
            "lifetime_referrer_fee_percentage": 3000,
            "referrer_rewards_percentage": 0,
            "name": "committee-account",
            "owner": {
                "weight_threshold": 1,
                "account_auths": [],
                "key_auths": [["BTS1111111111111111111111111111111114T1Anm", 1]],
                "address_auths": []
            },
            "active": {
                "weight_threshold": 1,
                "account_auths": [],
                "key_auths": [["BTS1111111111111111111111111111111114T1Anm", 1]],
                "address_auths": []
            },
            "options": {
                "memo_key": "BTS1111111111111111111111111111111114T1Anm",
                "voting_account": "1.2.5",
                "num_witness": 0,
                "num_committee": 0,
                "votes": [],
                "extensions": []
            },
            "num_committee_voted": 0,
            "statistics": "2.6.0",
            "whitelisting_accounts": [],
            "whitelisted_accounts": [],
            "blacklisted_accounts": [],
            "blacklisting_accounts": [],
            "cashback_vb": null,
            "owner_special_authority": [0, {}],
            "active_special_authority": [0, {}],
            "top_n_control_flags": 0,
            "allowed_assets": null,
            "creation_block_num": 1,
            "creation_time": "2026-05-26T12:00:00"
        })
    }

    #[test]
    fn generated_account_object_deserializes_from_get_objects_slot() {
        let requested_account_id = AccountIdRef::parse("1.2.0").unwrap();
        let result = serde_json::json!([account_object_json("1.2.0")]);

        let account = account_object_from_get_objects_result(&requested_account_id, result)
            .expect("parse account object")
            .expect("account should exist");

        assert_eq!(account.id, AccountId("1.2.0".to_string()));
        assert_eq!(account.name, "committee-account");
        assert_eq!(account.active.key_auths.len(), 1);
        assert_eq!(
            account.active.key_auths[0].0,
            "BTS1111111111111111111111111111111114T1Anm"
        );
    }

    #[test]
    fn generated_account_object_preserves_missing_get_objects_slot() {
        let requested_account_id = AccountIdRef::parse("1.2.999").unwrap();
        let result = serde_json::json!([null]);

        let account = account_object_from_get_objects_result(&requested_account_id, result)
            .expect("parse missing account slot");

        assert!(account.is_none());
    }

    #[test]
    fn generated_account_object_rejects_mismatched_get_objects_id() {
        let requested_account_id = AccountIdRef::parse("1.2.0").unwrap();
        let result = serde_json::json!([account_object_json("1.2.1")]);

        let error =
            account_object_from_get_objects_result(&requested_account_id, result).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("get_objects account id mismatch: expected 1.2.0, got 1.2.1")
        );
    }

    #[test]
    fn generated_account_object_rejects_wrong_get_objects_slot_count() {
        let requested_account_id = AccountIdRef::parse("1.2.0").unwrap();
        let result = serde_json::json!([account_object_json("1.2.0"), null]);

        let error =
            account_object_from_get_objects_result(&requested_account_id, result).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("get_objects account response must contain exactly one slot, got 2")
        );
    }

    #[test]
    fn generated_account_object_can_be_constructed_as_generated_type() {
        let account: AccountObject = serde_json::from_value(account_object_json("1.2.0")).unwrap();

        assert_eq!(account.id, AccountId("1.2.0".to_string()));
        assert_eq!(account.active.key_auths.len(), 1);
    }
}
