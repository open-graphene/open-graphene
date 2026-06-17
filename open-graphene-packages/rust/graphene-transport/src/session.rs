use std::time::Duration;

use serde_json::{Value, json};

use crate::{
    ChainIdMismatch, JsonRpcInbound, PendingCallback, ReconnectPolicy, TransportError,
    WebSocketTransport, is_connection_error,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiIds {
    pub database: u64,
    pub history: Option<u64>,
    pub network_broadcast: Option<u64>,
    pub crypto: Option<u64>,
    pub orders: Option<u64>,
}

pub struct GrapheneSession {
    transport: WebSocketTransport,
    api_ids: ApiIds,
    chain_id: String,
    reconnect_policy: ReconnectPolicy,
}

impl GrapheneSession {
    pub fn connect(url: &str) -> Result<Self, TransportError> {
        let transport = WebSocketTransport::connect(url)?;
        Self::from_transport(transport)
    }

    pub fn from_transport(mut transport: WebSocketTransport) -> Result<Self, TransportError> {
        let (api_ids, chain_id) = establish(&mut transport, None)?;
        Ok(Self {
            transport,
            api_ids,
            chain_id,
            reconnect_policy: ReconnectPolicy::default(),
        })
    }

    /// How this session reconnects after a dropped connection (default: a few retries with backoff).
    pub fn reconnect_policy(&self) -> ReconnectPolicy {
        self.reconnect_policy
    }

    /// Set the reconnect policy; pass [`ReconnectPolicy::disabled`] to turn auto-reconnect off.
    pub fn set_reconnect_policy(&mut self, policy: ReconnectPolicy) {
        self.reconnect_policy = policy;
    }

    /// Re-dial the same node and re-establish the API ids, in place.
    ///
    /// Refuses to reconnect if the node now reports a different chain id. Note that live
    /// subscriptions are not resumed: re-subscribe after a reconnect if you need them.
    pub fn reconnect(&mut self) -> Result<(), TransportError> {
        let url = self.transport.url().to_string();
        let mut transport = WebSocketTransport::connect(&url)?;
        let (api_ids, _) = establish(&mut transport, Some(&self.chain_id))?;
        self.transport = transport;
        self.api_ids = api_ids;
        Ok(())
    }

    /// Run an idempotent call, reconnecting and retrying on a dropped connection per the policy.
    ///
    /// The API id is resolved fresh on each attempt, since a reconnect rediscovers it.
    fn call_with_reconnect(
        &mut self,
        select: impl Fn(&ApiIds) -> Result<u64, TransportError>,
        method: &str,
        params: Value,
    ) -> Result<Value, TransportError> {
        let mut attempt = 0;
        loop {
            let api_id = select(&self.api_ids)?;
            match self.transport.call(api_id, method, params.clone()) {
                Ok(value) => return Ok(value),
                Err(error)
                    if is_connection_error(&error)
                        && attempt < self.reconnect_policy.max_retries =>
                {
                    attempt += 1;
                    std::thread::sleep(self.reconnect_policy.backoff_delay(attempt));
                    self.reconnect()?;
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    pub fn api_ids(&self) -> &ApiIds {
        &self.api_ids
    }

    pub fn into_transport(self) -> WebSocketTransport {
        self.transport
    }

    pub fn into_live_transport(self) -> Result<crate::LiveTransport, TransportError> {
        self.transport.into_live()
    }

    pub fn database_call(&mut self, method: &str, params: Value) -> Result<Value, TransportError> {
        self.call_with_reconnect(|ids| Ok(ids.database), method, params)
    }

    pub fn next_notice(&mut self) -> Result<JsonRpcInbound, TransportError> {
        self.transport.next_notice()
    }

    pub fn history_call(&mut self, method: &str, params: Value) -> Result<Value, TransportError> {
        self.call_with_reconnect(
            |ids| {
                ids.history
                    .ok_or(TransportError::MissingApi { name: "history" })
            },
            method,
            params,
        )
    }

    pub fn crypto_call(&mut self, method: &str, params: Value) -> Result<Value, TransportError> {
        self.call_with_reconnect(
            |ids| {
                ids.crypto
                    .ok_or(TransportError::MissingApi { name: "crypto" })
            },
            method,
            params,
        )
    }

    pub fn orders_call(&mut self, method: &str, params: Value) -> Result<Value, TransportError> {
        self.call_with_reconnect(
            |ids| {
                ids.orders
                    .ok_or(TransportError::MissingApi { name: "orders" })
            },
            method,
            params,
        )
    }

    pub fn network_broadcast_call(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, TransportError> {
        let api_id = self
            .api_ids
            .network_broadcast
            .ok_or(TransportError::MissingApi {
                name: "network_broadcast",
            })?;
        self.transport.call(api_id, method, params)
    }

    pub fn network_broadcast_call_with_callback(
        &mut self,
        method: &str,
        params_after_callback: Value,
    ) -> Result<Value, TransportError> {
        let api_id = self
            .api_ids
            .network_broadcast
            .ok_or(TransportError::MissingApi {
                name: "network_broadcast",
            })?;
        self.transport
            .call_with_callback(api_id, method, params_after_callback)
    }

    pub fn network_broadcast_call_with_callback_timeout(
        &mut self,
        method: &str,
        params_after_callback: Value,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        let api_id = self
            .api_ids
            .network_broadcast
            .ok_or(TransportError::MissingApi {
                name: "network_broadcast",
            })?;
        self.transport
            .call_with_callback_timeout(api_id, method, params_after_callback, timeout)
    }

    pub fn network_broadcast_send_callback_request(
        &mut self,
        method: &str,
        params_after_callback: Value,
    ) -> Result<PendingCallback, TransportError> {
        let api_id = self
            .api_ids
            .network_broadcast
            .ok_or(TransportError::MissingApi {
                name: "network_broadcast",
            })?;
        self.transport
            .send_callback_request(api_id, method, params_after_callback)
    }

    pub fn network_broadcast_wait_callback_response_and_notice_timeout(
        &mut self,
        pending: PendingCallback,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        self.transport
            .wait_for_callback_response_and_notice_timeout(pending, timeout)
    }
}

/// Log in, discover the API ids and read the chain id on a fresh transport.
///
/// When `expected_chain_id` is set, a mismatch is rejected so a reconnect never silently lands on
/// a different chain.
fn establish(
    transport: &mut WebSocketTransport,
    expected_chain_id: Option<&str>,
) -> Result<(ApiIds, String), TransportError> {
    transport.call(1, "login", json!(["", ""]))?;

    let database = discover_required_api(transport, "database")?;
    let history = discover_optional_api(transport, "history")?;
    let network_broadcast = discover_optional_api(transport, "network_broadcast")?;
    let crypto = discover_optional_api(transport, "crypto")?;
    let orders = discover_optional_api(transport, "orders")?;
    let chain_id = parse_chain_id(transport.call(database, "get_chain_id", json!([]))?)?;

    if let Some(expected) = expected_chain_id
        && expected != chain_id
    {
        return Err(TransportError::ChainIdMismatch {
            mismatch: ChainIdMismatch {
                server: transport.url().to_string(),
                expected: expected.to_string(),
                actual: chain_id,
            },
        });
    }

    Ok((
        ApiIds {
            database,
            history,
            network_broadcast,
            crypto,
            orders,
        },
        chain_id,
    ))
}

fn discover_required_api(
    transport: &mut WebSocketTransport,
    name: &'static str,
) -> Result<u64, TransportError> {
    let value = transport.call(1, name, json!([]))?;
    parse_api_id(name, value)
}

fn discover_optional_api(
    transport: &mut WebSocketTransport,
    name: &'static str,
) -> Result<Option<u64>, TransportError> {
    match transport.call(1, name, json!([])) {
        Ok(value) => parse_api_id(name, value).map(Some),
        Err(TransportError::RpcError { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn parse_api_id(name: &'static str, value: Value) -> Result<u64, TransportError> {
    value
        .as_u64()
        .ok_or(TransportError::InvalidApiId { name, value })
}

pub fn parse_chain_id(value: Value) -> Result<String, TransportError> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or(TransportError::InvalidChainId { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_api_id() {
        assert_eq!(parse_api_id("database", json!(2)).unwrap(), 2);
    }

    #[test]
    fn rejects_non_integer_api_id() {
        assert!(matches!(
            parse_api_id("database", json!("2")),
            Err(TransportError::InvalidApiId {
                name: "database",
                ..
            })
        ));
    }

    #[test]
    fn parses_chain_id() {
        assert_eq!(parse_chain_id(json!("abc123")).unwrap(), "abc123");
    }

    #[test]
    fn rejects_non_string_chain_id() {
        assert!(matches!(
            parse_chain_id(json!(123)),
            Err(TransportError::InvalidChainId { .. })
        ));
    }
}
