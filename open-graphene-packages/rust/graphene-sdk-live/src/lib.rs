use std::marker::PhantomData;

use open_graphene_sdk_core::HeadBlock;
use open_graphene_sdk_primitives::{AccountIdRef, ObjectIdError};
use open_graphene_transport::{get_objects, lookup_accounts, GrapheneSession, TransportError};
use serde_json::Value;
use thiserror::Error;

pub trait GrapheneChainProfile {
    const CORE_ASSET_ID: &'static str;
    const PUBLIC_KEY_PREFIX: &'static str;

    fn expected_chain_id() -> Option<&'static str> {
        None
    }
}

#[derive(Debug, Error)]
pub enum LiveSdkError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("connected chain id mismatch: expected {expected}, got {actual}")]
    ChainIdMismatch { expected: String, actual: String },
    #[error("account not found: {account_name}")]
    AccountNotFound { account_name: String },
    #[error(transparent)]
    ObjectId(#[from] ObjectIdError),
    #[error("invalid dynamic global properties object: {reason}: {value}")]
    InvalidDynamicGlobalProperties { reason: &'static str, value: Value },
    #[error("invalid account lookup result: {reason}: {value}")]
    InvalidAccountLookup { reason: &'static str, value: Value },
}

pub struct GrapheneLiveClient<P> {
    session: GrapheneSession,
    _profile: PhantomData<P>,
}

impl<P: GrapheneChainProfile> GrapheneLiveClient<P> {
    pub fn connect(url: &str) -> Result<Self, LiveSdkError> {
        Self::from_session(GrapheneSession::connect(url)?)
    }

    pub fn from_session(session: GrapheneSession) -> Result<Self, LiveSdkError> {
        validate_chain_id::<P>(session.chain_id())?;
        Ok(Self {
            session,
            _profile: PhantomData,
        })
    }

    pub fn chain_id(&self) -> &str {
        self.session.chain_id()
    }

    pub fn session(&self) -> &GrapheneSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut GrapheneSession {
        &mut self.session
    }

    pub fn head_block(&mut self) -> Result<HeadBlock, LiveSdkError> {
        head_block(&mut self.session)
    }

    pub fn lookup_account_id(&mut self, account_name: &str) -> Result<AccountIdRef, LiveSdkError> {
        lookup_account_id(&mut self.session, account_name)
    }

    pub fn lookup_account_id_optional(
        &mut self,
        account_name: &str,
    ) -> Result<Option<AccountIdRef>, LiveSdkError> {
        lookup_account_id_optional(&mut self.session, account_name)
    }
}

pub fn head_block(session: &mut GrapheneSession) -> Result<HeadBlock, LiveSdkError> {
    parse_head_block_from_get_objects(get_objects(session, ["2.1.0"])?)
}

pub fn lookup_account_id(
    session: &mut GrapheneSession,
    account_name: &str,
) -> Result<AccountIdRef, LiveSdkError> {
    lookup_account_id_optional(session, account_name)?.ok_or_else(|| {
        LiveSdkError::AccountNotFound {
            account_name: account_name.to_string(),
        }
    })
}

pub fn lookup_account_id_optional(
    session: &mut GrapheneSession,
    account_name: &str,
) -> Result<Option<AccountIdRef>, LiveSdkError> {
    parse_lookup_account_response(lookup_accounts(session, account_name, 1)?, account_name)
}

fn validate_chain_id<P: GrapheneChainProfile>(actual: &str) -> Result<(), LiveSdkError> {
    match P::expected_chain_id() {
        Some(expected) if expected != actual => Err(LiveSdkError::ChainIdMismatch {
            expected: expected.to_string(),
            actual: actual.to_string(),
        }),
        _ => Ok(()),
    }
}

fn parse_head_block_from_get_objects(value: Value) -> Result<HeadBlock, LiveSdkError> {
    let dynamic_global_properties = value.get(0).ok_or_else(|| {
        invalid_dynamic_global_properties(
            "dynamic global properties object was not returned",
            value.clone(),
        )
    })?;

    let number = dynamic_global_properties
        .get("head_block_number")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            invalid_dynamic_global_properties(
                "missing head_block_number",
                dynamic_global_properties.clone(),
            )
        })?;
    let id = dynamic_global_properties
        .get("head_block_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            invalid_dynamic_global_properties(
                "missing head_block_id",
                dynamic_global_properties.clone(),
            )
        })?
        .to_string();
    let time = dynamic_global_properties
        .get("time")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            invalid_dynamic_global_properties("missing time", dynamic_global_properties.clone())
        })?
        .to_string();

    Ok(HeadBlock { number, id, time })
}

fn invalid_dynamic_global_properties(reason: &'static str, value: Value) -> LiveSdkError {
    LiveSdkError::InvalidDynamicGlobalProperties { reason, value }
}

fn parse_lookup_account_response(
    value: Value,
    account_name: &str,
) -> Result<Option<AccountIdRef>, LiveSdkError> {
    let Some(pair) = value.as_array().and_then(|values| values.first()) else {
        return Ok(None);
    };

    let returned_name = pair
        .get(0)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_account_lookup("missing account name", pair.clone()))?;
    if returned_name != account_name {
        return Ok(None);
    }

    let account_id = pair
        .get(1)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_account_lookup("missing account id", pair.clone()))?;

    Ok(Some(AccountIdRef::parse(account_id)?))
}

fn invalid_account_lookup(reason: &'static str, value: Value) -> LiveSdkError {
    LiveSdkError::InvalidAccountLookup { reason, value }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct StrictProfile;

    impl GrapheneChainProfile for StrictProfile {
        const CORE_ASSET_ID: &'static str = "1.3.0";
        const PUBLIC_KEY_PREFIX: &'static str = "BTS";

        fn expected_chain_id() -> Option<&'static str> {
            Some("expected-chain-id")
        }
    }

    struct AnyChainProfile;

    impl GrapheneChainProfile for AnyChainProfile {
        const CORE_ASSET_ID: &'static str = "1.3.0";
        const PUBLIC_KEY_PREFIX: &'static str = "BTS";
    }

    #[test]
    fn accepts_matching_chain_id() {
        validate_chain_id::<StrictProfile>("expected-chain-id").unwrap();
    }

    #[test]
    fn rejects_mismatched_chain_id() {
        let error = validate_chain_id::<StrictProfile>("actual-chain-id").unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::ChainIdMismatch { expected, actual }
                if expected == "expected-chain-id" && actual == "actual-chain-id"
        ));
    }

    #[test]
    fn accepts_any_chain_when_profile_has_no_expected_chain_id() {
        validate_chain_id::<AnyChainProfile>("actual-chain-id").unwrap();
    }

    #[test]
    fn parses_head_block_from_dynamic_global_properties_response() {
        let head = parse_head_block_from_get_objects(json!([{
            "head_block_number": 609782,
            "head_block_id": "00094df644fe617490ae116a0100400d03000000",
            "time": "2026-05-25T12:00:00"
        }]))
        .unwrap();

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
    fn rejects_empty_get_objects_response() {
        let error = parse_head_block_from_get_objects(json!([])).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidDynamicGlobalProperties { reason, .. }
                if reason == "dynamic global properties object was not returned"
        ));
    }

    #[test]
    fn rejects_missing_head_block_number() {
        let error = parse_head_block_from_get_objects(json!([{
            "head_block_id": "00094df644fe617490ae116a0100400d03000000",
            "time": "2026-05-25T12:00:00"
        }]))
        .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidDynamicGlobalProperties { reason, .. }
                if reason == "missing head_block_number"
        ));
    }

    #[test]
    fn rejects_missing_head_block_id() {
        let error = parse_head_block_from_get_objects(json!([{
            "head_block_number": 609782,
            "time": "2026-05-25T12:00:00"
        }]))
        .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidDynamicGlobalProperties { reason, .. }
                if reason == "missing head_block_id"
        ));
    }

    #[test]
    fn rejects_missing_time() {
        let error = parse_head_block_from_get_objects(json!([{
            "head_block_number": 609782,
            "head_block_id": "00094df644fe617490ae116a0100400d03000000"
        }]))
        .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidDynamicGlobalProperties { reason, .. }
                if reason == "missing time"
        ));
    }

    #[test]
    fn parses_exact_account_lookup_response() {
        let account_id = parse_lookup_account_response(json!([["alice", "1.2.100"]]), "alice")
            .unwrap()
            .unwrap();

        assert_eq!(account_id.to_string(), "1.2.100");
    }

    #[test]
    fn account_lookup_returns_none_for_empty_result() {
        assert_eq!(
            parse_lookup_account_response(json!([]), "alice").unwrap(),
            None
        );
    }

    #[test]
    fn account_lookup_returns_none_for_non_exact_name() {
        assert_eq!(
            parse_lookup_account_response(json!([["alice2", "1.2.100"]]), "alice").unwrap(),
            None
        );
    }

    #[test]
    fn account_lookup_rejects_missing_account_name() {
        let error = parse_lookup_account_response(json!([[null, "1.2.100"]]), "alice").unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidAccountLookup { reason, .. }
                if reason == "missing account name"
        ));
    }

    #[test]
    fn account_lookup_rejects_missing_account_id() {
        let error = parse_lookup_account_response(json!([["alice"]]), "alice").unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidAccountLookup { reason, .. }
                if reason == "missing account id"
        ));
    }

    #[test]
    fn account_lookup_rejects_non_account_object_id() {
        let error =
            parse_lookup_account_response(json!([["alice", "1.3.0"]]), "alice").unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::ObjectId(ObjectIdError::UnexpectedType { .. })
        ));
    }
}
