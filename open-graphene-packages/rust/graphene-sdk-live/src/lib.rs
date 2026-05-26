use std::marker::PhantomData;

use open_graphene_sdk_core::HeadBlock;
use open_graphene_sdk_primitives::{
    AccountIdRef, AssetAmount, AssetIdRef, LimitOrderIdRef, ObjectIdError,
};
use open_graphene_transport::{
    get_account_balances, get_limit_orders as transport_get_limit_orders, get_objects,
    get_required_fees, lookup_accounts, lookup_asset_symbols, GrapheneSession, TransportError,
};
use serde_json::Value;
use thiserror::Error;

pub trait GrapheneChainProfile {
    const CORE_ASSET_ID: &'static str;
    const PUBLIC_KEY_PREFIX: &'static str;

    fn expected_chain_id() -> Option<&'static str> {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOrderSummary {
    pub id: LimitOrderIdRef,
    pub seller: AccountIdRef,
}

#[derive(Debug, Error)]
pub enum LiveSdkError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("connected chain id mismatch: expected {expected}, got {actual}")]
    ChainIdMismatch { expected: String, actual: String },
    #[error("account not found: {account_name}")]
    AccountNotFound { account_name: String },
    #[error("asset not found: {symbol}")]
    AssetNotFound { symbol: String },
    #[error(transparent)]
    ObjectId(#[from] ObjectIdError),
    #[error("invalid dynamic global properties object: {reason}: {value}")]
    InvalidDynamicGlobalProperties { reason: &'static str, value: Value },
    #[error("invalid account lookup result: {reason}: {value}")]
    InvalidAccountLookup { reason: &'static str, value: Value },
    #[error("invalid asset lookup result: {reason}: {value}")]
    InvalidAssetLookup { reason: &'static str, value: Value },
    #[error("invalid account balance result: {reason}: {value}")]
    InvalidAccountBalance { reason: &'static str, value: Value },
    #[error("invalid required fee result: {reason}: {value}")]
    InvalidRequiredFee { reason: &'static str, value: Value },
    #[error("invalid limit orders result: {reason}: {value}")]
    InvalidLimitOrders { reason: &'static str, value: Value },
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

    pub fn lookup_asset_id(&mut self, symbol: &str) -> Result<AssetIdRef, LiveSdkError> {
        lookup_asset_id(&mut self.session, symbol)
    }

    pub fn lookup_asset_id_optional(
        &mut self,
        symbol: &str,
    ) -> Result<Option<AssetIdRef>, LiveSdkError> {
        lookup_asset_id_optional(&mut self.session, symbol)
    }

    pub fn account_balance(
        &mut self,
        account_id: &AccountIdRef,
        asset_id: &AssetIdRef,
    ) -> Result<AssetAmount, LiveSdkError> {
        account_balance(&mut self.session, account_id, asset_id)
    }

    pub fn required_fee_for_operation_json(
        &mut self,
        operation_json: Value,
        fee_asset_id: &AssetIdRef,
    ) -> Result<AssetAmount, LiveSdkError> {
        required_fee_for_operation_json(&mut self.session, operation_json, fee_asset_id)
    }

    pub fn limit_orders(
        &mut self,
        base_asset_id: &AssetIdRef,
        quote_asset_id: &AssetIdRef,
        limit: u64,
    ) -> Result<Vec<LimitOrderSummary>, LiveSdkError> {
        limit_orders(&mut self.session, base_asset_id, quote_asset_id, limit)
    }

    pub fn limit_order_exists(&mut self, order_id: &LimitOrderIdRef) -> Result<bool, LiveSdkError> {
        limit_order_exists(&mut self.session, order_id)
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

pub fn lookup_asset_id(
    session: &mut GrapheneSession,
    symbol: &str,
) -> Result<AssetIdRef, LiveSdkError> {
    lookup_asset_id_optional(session, symbol)?.ok_or_else(|| LiveSdkError::AssetNotFound {
        symbol: symbol.to_string(),
    })
}

pub fn lookup_asset_id_optional(
    session: &mut GrapheneSession,
    symbol: &str,
) -> Result<Option<AssetIdRef>, LiveSdkError> {
    parse_lookup_asset_response(lookup_asset_symbols(session, [symbol])?, symbol)
}

pub fn account_balance(
    session: &mut GrapheneSession,
    account_id: &AccountIdRef,
    asset_id: &AssetIdRef,
) -> Result<AssetAmount, LiveSdkError> {
    parse_account_balance_response(
        get_account_balances(session, account_id.to_string(), [asset_id.to_string()])?,
        asset_id,
    )
}

pub fn required_fee_for_operation_json(
    session: &mut GrapheneSession,
    operation_json: Value,
    fee_asset_id: &AssetIdRef,
) -> Result<AssetAmount, LiveSdkError> {
    parse_required_fee_response(
        get_required_fees(
            session,
            Value::Array(vec![operation_json]),
            fee_asset_id.to_string(),
        )?,
        fee_asset_id,
    )
}

pub fn limit_orders(
    session: &mut GrapheneSession,
    base_asset_id: &AssetIdRef,
    quote_asset_id: &AssetIdRef,
    limit: u64,
) -> Result<Vec<LimitOrderSummary>, LiveSdkError> {
    parse_limit_orders_response(transport_get_limit_orders(
        session,
        base_asset_id.to_string(),
        quote_asset_id.to_string(),
        limit,
    )?)
}

pub fn limit_order_exists(
    session: &mut GrapheneSession,
    order_id: &LimitOrderIdRef,
) -> Result<bool, LiveSdkError> {
    parse_limit_order_exists_response(get_objects(session, [order_id.to_string()])?)
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

fn parse_lookup_asset_response(
    value: Value,
    symbol: &str,
) -> Result<Option<AssetIdRef>, LiveSdkError> {
    let Some(asset) = value.as_array().and_then(|values| values.first()) else {
        return Ok(None);
    };
    if asset.is_null() {
        return Ok(None);
    }

    let returned_symbol = asset
        .get("symbol")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_asset_lookup("missing symbol", asset.clone()))?;
    if returned_symbol != symbol {
        return Ok(None);
    }

    let asset_id = asset
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_asset_lookup("missing id", asset.clone()))?;

    Ok(Some(AssetIdRef::parse(asset_id)?))
}

fn invalid_asset_lookup(reason: &'static str, value: Value) -> LiveSdkError {
    LiveSdkError::InvalidAssetLookup { reason, value }
}

fn parse_limit_orders_response(value: Value) -> Result<Vec<LimitOrderSummary>, LiveSdkError> {
    let orders = value
        .as_array()
        .ok_or_else(|| invalid_limit_orders("result is not an array", value.clone()))?;

    orders
        .iter()
        .map(|order| {
            let id = order
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_limit_orders("missing id", order.clone()))?;
            let seller = order
                .get("seller")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_limit_orders("missing seller", order.clone()))?;

            Ok(LimitOrderSummary {
                id: LimitOrderIdRef::parse(id)?,
                seller: AccountIdRef::parse(seller)?,
            })
        })
        .collect()
}

fn invalid_limit_orders(reason: &'static str, value: Value) -> LiveSdkError {
    LiveSdkError::InvalidLimitOrders { reason, value }
}

fn parse_limit_order_exists_response(value: Value) -> Result<bool, LiveSdkError> {
    let order = value
        .as_array()
        .and_then(|values| values.first())
        .ok_or_else(|| invalid_limit_orders("missing limit order object", value.clone()))?;

    if order.is_null() {
        return Ok(false);
    }
    if order.is_object() {
        return Ok(true);
    }

    Err(invalid_limit_orders(
        "limit order object is not an object or null",
        order.clone(),
    ))
}

fn parse_account_balance_response(
    value: Value,
    expected_asset_id: &AssetIdRef,
) -> Result<AssetAmount, LiveSdkError> {
    let balance = value
        .as_array()
        .and_then(|values| values.first())
        .ok_or_else(|| invalid_account_balance("missing balance", value.clone()))?;

    parse_asset_amount_object(balance, expected_asset_id, invalid_account_balance)
}

fn parse_required_fee_response(
    value: Value,
    expected_asset_id: &AssetIdRef,
) -> Result<AssetAmount, LiveSdkError> {
    let fee = value
        .as_array()
        .and_then(|values| values.first())
        .ok_or_else(|| invalid_required_fee("missing fee", value.clone()))?;

    parse_asset_amount_object(fee, expected_asset_id, invalid_required_fee)
}

fn parse_asset_amount_object(
    value: &Value,
    expected_asset_id: &AssetIdRef,
    invalid: fn(&'static str, Value) -> LiveSdkError,
) -> Result<AssetAmount, LiveSdkError> {
    let returned_asset_id = value
        .get("asset_id")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("missing asset_id", value.clone()))?;
    let returned_asset_id = AssetIdRef::parse(returned_asset_id)?;
    if &returned_asset_id != expected_asset_id {
        return Err(invalid("unexpected asset_id", value.clone()));
    }

    let amount = value
        .get("amount")
        .and_then(json_i64)
        .ok_or_else(|| invalid("missing integer amount", value.clone()))?;

    Ok(AssetAmount::new(amount, returned_asset_id))
}

fn invalid_required_fee(reason: &'static str, value: Value) -> LiveSdkError {
    LiveSdkError::InvalidRequiredFee { reason, value }
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn invalid_account_balance(reason: &'static str, value: Value) -> LiveSdkError {
    LiveSdkError::InvalidAccountBalance { reason, value }
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

    #[test]
    fn parses_exact_asset_lookup_response() {
        let asset_id =
            parse_lookup_asset_response(json!([{"id": "1.3.0", "symbol": "BTS"}]), "BTS")
                .unwrap()
                .unwrap();

        assert_eq!(asset_id.to_string(), "1.3.0");
    }

    #[test]
    fn asset_lookup_returns_none_for_empty_result() {
        assert_eq!(parse_lookup_asset_response(json!([]), "BTS").unwrap(), None);
    }

    #[test]
    fn asset_lookup_returns_none_for_null_result() {
        assert_eq!(
            parse_lookup_asset_response(json!([null]), "BTS").unwrap(),
            None
        );
    }

    #[test]
    fn asset_lookup_returns_none_for_non_exact_symbol() {
        assert_eq!(
            parse_lookup_asset_response(json!([{"id": "1.3.0", "symbol": "BTST"}]), "BTS").unwrap(),
            None
        );
    }

    #[test]
    fn asset_lookup_rejects_missing_symbol() {
        let error = parse_lookup_asset_response(json!([{"id": "1.3.0"}]), "BTS").unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidAssetLookup { reason, .. } if reason == "missing symbol"
        ));
    }

    #[test]
    fn asset_lookup_rejects_missing_id() {
        let error = parse_lookup_asset_response(json!([{"symbol": "BTS"}]), "BTS").unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidAssetLookup { reason, .. } if reason == "missing id"
        ));
    }

    #[test]
    fn asset_lookup_rejects_non_asset_object_id() {
        let error = parse_lookup_asset_response(json!([{"id": "1.2.100", "symbol": "BTS"}]), "BTS")
            .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::ObjectId(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn parses_account_balance_with_integer_amount() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let balance =
            parse_account_balance_response(json!([{"amount": 42, "asset_id": "1.3.0"}]), &asset_id)
                .unwrap();

        assert_eq!(balance, AssetAmount::new(42, asset_id));
    }

    #[test]
    fn parses_account_balance_with_string_amount() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let balance = parse_account_balance_response(
            json!([{"amount": "42", "asset_id": "1.3.0"}]),
            &asset_id,
        )
        .unwrap();

        assert_eq!(balance, AssetAmount::new(42, asset_id));
    }

    #[test]
    fn account_balance_rejects_missing_balance() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error = parse_account_balance_response(json!([]), &asset_id).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidAccountBalance { reason, .. } if reason == "missing balance"
        ));
    }

    #[test]
    fn account_balance_rejects_missing_asset_id() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error = parse_account_balance_response(json!([{"amount": 42}]), &asset_id).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidAccountBalance { reason, .. } if reason == "missing asset_id"
        ));
    }

    #[test]
    fn account_balance_rejects_unexpected_asset_id() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error =
            parse_account_balance_response(json!([{"amount": 42, "asset_id": "1.3.1"}]), &asset_id)
                .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidAccountBalance { reason, .. } if reason == "unexpected asset_id"
        ));
    }

    #[test]
    fn account_balance_rejects_non_asset_object_id() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error = parse_account_balance_response(
            json!([{"amount": 42, "asset_id": "1.2.100"}]),
            &asset_id,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::ObjectId(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn account_balance_rejects_malformed_amount() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error = parse_account_balance_response(
            json!([{"amount": "nope", "asset_id": "1.3.0"}]),
            &asset_id,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidAccountBalance { reason, .. } if reason == "missing integer amount"
        ));
    }

    #[test]
    fn parses_required_fee_with_integer_amount() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let fee =
            parse_required_fee_response(json!([{"amount": 7, "asset_id": "1.3.0"}]), &asset_id)
                .unwrap();

        assert_eq!(fee, AssetAmount::new(7, asset_id));
    }

    #[test]
    fn parses_required_fee_with_string_amount() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let fee =
            parse_required_fee_response(json!([{"amount": "8", "asset_id": "1.3.0"}]), &asset_id)
                .unwrap();

        assert_eq!(fee, AssetAmount::new(8, asset_id));
    }

    #[test]
    fn required_fee_rejects_missing_fee() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error = parse_required_fee_response(json!([]), &asset_id).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidRequiredFee { reason, .. } if reason == "missing fee"
        ));
    }

    #[test]
    fn required_fee_rejects_missing_asset_id() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error = parse_required_fee_response(json!([{"amount": 7}]), &asset_id).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidRequiredFee { reason, .. } if reason == "missing asset_id"
        ));
    }

    #[test]
    fn required_fee_rejects_unexpected_asset_id() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error =
            parse_required_fee_response(json!([{"amount": 7, "asset_id": "1.3.1"}]), &asset_id)
                .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidRequiredFee { reason, .. } if reason == "unexpected asset_id"
        ));
    }

    #[test]
    fn required_fee_rejects_non_asset_object_id() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error =
            parse_required_fee_response(json!([{"amount": 7, "asset_id": "1.2.100"}]), &asset_id)
                .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::ObjectId(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn required_fee_rejects_malformed_amount() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let error = parse_required_fee_response(
            json!([{"amount": "nope", "asset_id": "1.3.0"}]),
            &asset_id,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidRequiredFee { reason, .. } if reason == "missing integer amount"
        ));
    }

    #[test]
    fn parses_limit_orders_with_id_and_seller() {
        let orders = parse_limit_orders_response(json!([
            {"id": "1.7.42", "seller": "1.2.100", "for_sale": 5}
        ]))
        .unwrap();

        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].id.to_string(), "1.7.42");
        assert_eq!(orders[0].seller.to_string(), "1.2.100");
    }

    #[test]
    fn parses_empty_limit_orders_response() {
        assert_eq!(parse_limit_orders_response(json!([])).unwrap(), Vec::new());
    }

    #[test]
    fn limit_orders_rejects_non_array_result() {
        let error = parse_limit_orders_response(json!({"id": "1.7.42"})).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidLimitOrders { reason, .. } if reason == "result is not an array"
        ));
    }

    #[test]
    fn limit_orders_rejects_missing_id() {
        let error = parse_limit_orders_response(json!([{"seller": "1.2.100"}])).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidLimitOrders { reason, .. } if reason == "missing id"
        ));
    }

    #[test]
    fn limit_orders_rejects_missing_seller() {
        let error = parse_limit_orders_response(json!([{"id": "1.7.42"}])).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidLimitOrders { reason, .. } if reason == "missing seller"
        ));
    }

    #[test]
    fn limit_orders_rejects_non_limit_order_id() {
        let error = parse_limit_orders_response(json!([{"id": "1.2.100", "seller": "1.2.100"}]))
            .unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::ObjectId(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn limit_orders_rejects_non_account_seller_id() {
        let error =
            parse_limit_orders_response(json!([{"id": "1.7.42", "seller": "1.3.0"}])).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::ObjectId(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn limit_order_exists_returns_true_for_object() {
        assert!(parse_limit_order_exists_response(json!([{"id": "1.7.42"}])).unwrap());
    }

    #[test]
    fn limit_order_exists_returns_false_for_null() {
        assert!(!parse_limit_order_exists_response(json!([null])).unwrap());
    }

    #[test]
    fn limit_order_exists_rejects_empty_response() {
        let error = parse_limit_order_exists_response(json!([])).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidLimitOrders { reason, .. } if reason == "missing limit order object"
        ));
    }

    #[test]
    fn limit_order_exists_rejects_non_array_response() {
        let error = parse_limit_order_exists_response(json!({"id": "1.7.42"})).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidLimitOrders { reason, .. } if reason == "missing limit order object"
        ));
    }

    #[test]
    fn limit_order_exists_rejects_non_object_response() {
        let error = parse_limit_order_exists_response(json!(["1.7.42"])).unwrap_err();

        assert!(matches!(
            error,
            LiveSdkError::InvalidLimitOrders { reason, .. } if reason == "limit order object is not an object or null"
        ));
    }
}
