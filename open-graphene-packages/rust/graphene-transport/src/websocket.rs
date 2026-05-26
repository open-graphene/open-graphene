use std::collections::VecDeque;
use std::net::TcpStream;

use serde_json::Value;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

use crate::{parse_inbound, JsonRpcInbound, JsonRpcRequest, TransportError};

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

    pub fn next_buffered_notice(&mut self) -> Option<JsonRpcInbound> {
        self.buffered_notices.pop_front()
    }

    pub fn buffered_notice_count(&self) -> usize {
        self.buffered_notices.len()
    }

    fn allocate_request_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
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
