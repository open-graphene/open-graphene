use std::collections::VecDeque;
use std::net::TcpStream;

use serde_json::{Value, json};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket, connect};

use crate::{JsonRpcInbound, JsonRpcRequest, TransportError, parse_inbound};

pub struct WebSocketTransport {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    next_id: u64,
    buffered_notices: VecDeque<JsonRpcInbound>,
}

impl WebSocketTransport {
    pub fn connect(url: &str) -> Result<Self, TransportError> {
        let (socket, _) = connect(url).map_err(TransportError::websocket)?;
        Ok(Self {
            socket,
            next_id: 1,
            buffered_notices: VecDeque::new(),
        })
    }

    pub fn call(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, TransportError> {
        let id = self.allocate_request_id();
        let request = JsonRpcRequest::graphene_call(id, api_id, method, params);
        self.socket
            .send(Message::Text(request.to_value().to_string()))
            .map_err(TransportError::websocket)?;

        self.wait_for_response(id)
    }

    pub fn call_with_callback(
        &mut self,
        api_id: u64,
        method: &str,
        params_after_callback: Value,
    ) -> Result<Value, TransportError> {
        let id = self.allocate_request_id();
        let params = params_with_callback_id(id, params_after_callback)?;
        let request = JsonRpcRequest::graphene_call(id, api_id, method, params);
        self.socket
            .send(Message::Text(request.to_value().to_string()))
            .map_err(TransportError::websocket)?;

        self.wait_for_response(id)?;
        self.wait_for_notice_payload(id)
    }

    pub fn next_buffered_notice(&mut self) -> Option<JsonRpcInbound> {
        self.buffered_notices.pop_front()
    }

    pub fn next_notice(&mut self) -> Result<JsonRpcInbound, TransportError> {
        if let Some(notice) = self.next_buffered_notice() {
            return Ok(notice);
        }

        loop {
            match self.read_inbound()? {
                notice @ JsonRpcInbound::Notice { .. } => return Ok(notice),
                JsonRpcInbound::Response { .. } | JsonRpcInbound::Error { .. } => {
                    return Err(TransportError::UnsupportedInboundMessage);
                }
            }
        }
    }

    pub fn buffered_notice_count(&self) -> usize {
        self.buffered_notices.len()
    }

    fn allocate_request_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn wait_for_response(&mut self, id: u64) -> Result<Value, TransportError> {
        loop {
            match self.read_inbound()? {
                JsonRpcInbound::Response { id: actual, result } if actual == id => {
                    return Ok(result);
                }
                JsonRpcInbound::Error { id: actual, error } if actual == id => {
                    return Err(TransportError::RpcError { id: actual, error });
                }
                JsonRpcInbound::Response { id: actual, .. }
                | JsonRpcInbound::Error { id: actual, .. } => {
                    return Err(TransportError::UnexpectedResponseId {
                        expected: id,
                        actual,
                    });
                }
                notice @ JsonRpcInbound::Notice { .. } => {
                    self.buffered_notices.push_back(notice);
                }
            }
        }
    }

    fn wait_for_notice_payload(&mut self, callback_id: u64) -> Result<Value, TransportError> {
        if let Some(payload) = self.take_buffered_notice_payload(callback_id) {
            return Ok(payload);
        }

        loop {
            match self.read_inbound()? {
                JsonRpcInbound::Notice {
                    callback_id: actual,
                    payload,
                } if actual == callback_id => return Ok(payload),
                notice @ JsonRpcInbound::Notice { .. } => {
                    self.buffered_notices.push_back(notice);
                }
                JsonRpcInbound::Response { .. } | JsonRpcInbound::Error { .. } => {
                    return Err(TransportError::UnsupportedInboundMessage);
                }
            }
        }
    }

    fn take_buffered_notice_payload(&mut self, callback_id: u64) -> Option<Value> {
        let position = self
            .buffered_notices
            .iter()
            .position(|notice| matches!(notice, JsonRpcInbound::Notice { callback_id: actual, .. } if *actual == callback_id))?;
        let Some(JsonRpcInbound::Notice { payload, .. }) = self.buffered_notices.remove(position)
        else {
            return None;
        };
        Some(payload)
    }

    fn read_inbound(&mut self) -> Result<JsonRpcInbound, TransportError> {
        loop {
            match self.socket.read().map_err(TransportError::websocket)? {
                Message::Text(text) => {
                    let value = serde_json::from_str(&text).map_err(TransportError::Json)?;
                    return parse_inbound(&value);
                }
                Message::Close(_) => return Err(TransportError::ConnectionClosed),
                Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
                    continue;
                }
            }
        }
    }
}

fn params_with_callback_id(
    callback_id: u64,
    params_after_callback: Value,
) -> Result<Value, TransportError> {
    let mut params = params_after_callback
        .as_array()
        .cloned()
        .ok_or(TransportError::CallbackParamsNotArray)?;
    params.insert(0, json!(callback_id));
    Ok(Value::Array(params))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepends_graphene_callback_id_to_params_array() {
        assert_eq!(
            params_with_callback_id(7, json!([{"trx": true}])).unwrap(),
            json!([7, {"trx": true}])
        );
    }

    #[test]
    fn rejects_callback_params_that_are_not_an_array() {
        assert!(matches!(
            params_with_callback_id(7, json!({"trx": true})),
            Err(TransportError::CallbackParamsNotArray)
        ));
    }
}
