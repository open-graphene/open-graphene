use std::error::Error;

use serde_json::{json, Value};
use tungstenite::{connect, Message, WebSocket};

pub struct GrapheneRpc {
    socket: WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    next_id: u64,
}

impl GrapheneRpc {
    pub fn connect(url: &str) -> Result<Self, Box<dyn Error>> {
        let (socket, _) = connect(url)?;
        Ok(Self { socket, next_id: 1 })
    }

    pub fn call_raw(&mut self, params: Value) -> Result<Value, Box<dyn Error>> {
        let id = self.next_id;
        self.next_id += 1;
        self.socket.send(Message::Text(
            json!({"id": id, "method": "call", "params": params}).to_string(),
        ))?;
        loop {
            let response = self.socket.read()?;
            let Message::Text(text) = response else {
                continue;
            };
            let value: Value = serde_json::from_str(&text)?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(format!("Graphene RPC error: {error}").into());
            }
            return value
                .get("result")
                .cloned()
                .ok_or_else(|| "Graphene RPC response missing result".into());
        }
    }

    pub fn database_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        self.call_raw(json!([1, "database", []]))?
            .as_u64()
            .ok_or_else(|| "database API id is not an integer".into())
    }

    pub fn network_broadcast_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        self.call_raw(json!([1, "network_broadcast", []]))?
            .as_u64()
            .ok_or_else(|| "network_broadcast API id is not an integer".into())
    }

    pub fn history_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        self.call_raw(json!([1, "history", []]))?
            .as_u64()
            .ok_or_else(|| "history API id is not an integer".into())
    }

    pub fn call_database(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        self.call_raw(json!([api_id, method, params]))
    }

    pub fn call_network_broadcast(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        self.call_raw(json!([api_id, method, params]))
    }

    pub fn call_history(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        self.call_raw(json!([api_id, method, params]))
    }
}
