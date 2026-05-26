use serde_json::{Value, json};

use crate::{JsonRpcInbound, TransportError, WebSocketTransport};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiIds {
    pub database: u64,
    pub history: Option<u64>,
    pub network_broadcast: Option<u64>,
}

pub struct GrapheneSession {
    transport: WebSocketTransport,
    api_ids: ApiIds,
    chain_id: String,
}

impl GrapheneSession {
    pub fn connect(url: &str) -> Result<Self, TransportError> {
        let transport = WebSocketTransport::connect(url)?;
        Self::from_transport(transport)
    }

    pub fn from_transport(mut transport: WebSocketTransport) -> Result<Self, TransportError> {
        transport.call(1, "login", json!(["", ""]))?;

        let database = discover_required_api(&mut transport, "database")?;
        let history = discover_optional_api(&mut transport, "history")?;
        let network_broadcast = discover_optional_api(&mut transport, "network_broadcast")?;
        let chain_id = parse_chain_id(transport.call(database, "get_chain_id", json!([]))?)?;

        Ok(Self {
            transport,
            api_ids: ApiIds {
                database,
                history,
                network_broadcast,
            },
            chain_id,
        })
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

    pub fn database_call(&mut self, method: &str, params: Value) -> Result<Value, TransportError> {
        self.transport.call(self.api_ids.database, method, params)
    }

    pub fn next_notice(&mut self) -> Result<JsonRpcInbound, TransportError> {
        self.transport.next_notice()
    }

    pub fn history_call(&mut self, method: &str, params: Value) -> Result<Value, TransportError> {
        let api_id = self
            .api_ids
            .history
            .ok_or(TransportError::MissingApi { name: "history" })?;
        self.transport.call(api_id, method, params)
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
