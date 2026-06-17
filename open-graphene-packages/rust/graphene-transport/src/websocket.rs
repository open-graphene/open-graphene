use std::collections::VecDeque;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Error as WebSocketError, Message, WebSocket, connect};

use crate::{CallbackId, JsonRpcInbound, JsonRpcRequest, TransportError, parse_inbound};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingCallback {
    callback_id: CallbackId,
}

impl PendingCallback {
    pub const fn new(callback_id: CallbackId) -> Self {
        Self { callback_id }
    }

    pub const fn callback_id(self) -> CallbackId {
        self.callback_id
    }
}

pub struct WebSocketTransport {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    url: String,
    next_id: u64,
    buffered_notices: VecDeque<JsonRpcInbound>,
    buffered_responses: VecDeque<JsonRpcInbound>,
}

impl WebSocketTransport {
    pub fn connect(url: &str) -> Result<Self, TransportError> {
        let (socket, _) = connect(url).map_err(TransportError::websocket)?;
        Ok(Self {
            socket,
            url: url.to_string(),
            next_id: 1,
            buffered_notices: VecDeque::new(),
            buffered_responses: VecDeque::new(),
        })
    }

    /// The node URL this transport dialed, so a session can reconnect to the same node.
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn call(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, TransportError> {
        let id = self.send_request(api_id, method, params)?;
        self.wait_for_response(id)
    }

    pub fn into_live(self) -> Result<crate::LiveTransport, TransportError> {
        crate::LiveTransport::spawn(self)
    }

    pub fn call_with_callback(
        &mut self,
        api_id: u64,
        method: &str,
        params_after_callback: Value,
    ) -> Result<Value, TransportError> {
        let pending = self.send_callback_request(api_id, method, params_after_callback)?;
        self.wait_for_callback_response_and_notice(pending)
    }

    pub fn call_with_callback_timeout(
        &mut self,
        api_id: u64,
        method: &str,
        params_after_callback: Value,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        let pending = self.send_callback_request(api_id, method, params_after_callback)?;
        self.wait_for_callback_response_and_notice_timeout(pending, timeout)
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
                response @ (JsonRpcInbound::Response { .. } | JsonRpcInbound::Error { .. }) => {
                    self.buffered_responses.push_back(response);
                }
            }
        }
    }

    pub fn buffered_notice_count(&self) -> usize {
        self.buffered_notices.len()
    }

    pub fn send_request(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<u64, TransportError> {
        let id = self.allocate_request_id();
        let request = JsonRpcRequest::graphene_call(id, api_id, method, params);
        self.socket
            .send(Message::Text(request.to_value().to_string()))
            .map_err(TransportError::websocket)?;
        Ok(id)
    }

    pub fn send_callback_request(
        &mut self,
        api_id: u64,
        method: &str,
        params_after_callback: Value,
    ) -> Result<PendingCallback, TransportError> {
        let callback_id = CallbackId::new(self.allocate_request_id());
        let params = params_with_callback(callback_id, params_after_callback)?;
        let request = JsonRpcRequest::graphene_call(callback_id.as_u64(), api_id, method, params);
        self.socket
            .send(Message::Text(request.to_value().to_string()))
            .map_err(TransportError::websocket)?;
        Ok(PendingCallback::new(callback_id))
    }

    pub fn wait_for_callback_response_and_notice(
        &mut self,
        pending: PendingCallback,
    ) -> Result<Value, TransportError> {
        let callback_id = pending.callback_id();
        self.wait_for_response(callback_id.as_u64())?;
        self.wait_for_callback_notice(callback_id)
    }

    pub fn wait_for_callback_response_and_notice_timeout(
        &mut self,
        pending: PendingCallback,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        let callback_id = pending.callback_id();
        self.wait_for_response(callback_id.as_u64())?;
        self.wait_for_callback_notice_with_timeout(callback_id, timeout)
    }

    fn allocate_request_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn wait_for_response(&mut self, id: u64) -> Result<Value, TransportError> {
        if let Some(response) = self.take_buffered_response(id) {
            return response;
        }

        loop {
            match self.read_inbound()? {
                JsonRpcInbound::Response { id: actual, result } if actual == id => {
                    return Ok(result);
                }
                JsonRpcInbound::Error { id: actual, error } if actual == id => {
                    return Err(TransportError::RpcError { id: actual, error });
                }
                response @ (JsonRpcInbound::Response { .. } | JsonRpcInbound::Error { .. }) => {
                    self.buffered_responses.push_back(response);
                }
                notice @ JsonRpcInbound::Notice { .. } => {
                    self.buffered_notices.push_back(notice);
                }
            }
        }
    }

    pub(crate) fn wait_for_callback_notice(
        &mut self,
        callback_id: CallbackId,
    ) -> Result<Value, TransportError> {
        if let Some(payload) = self.take_buffered_callback_notice(callback_id) {
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
                response @ (JsonRpcInbound::Response { .. } | JsonRpcInbound::Error { .. }) => {
                    self.buffered_responses.push_back(response);
                }
            }
        }
    }

    pub(crate) fn wait_for_callback_notice_with_timeout(
        &mut self,
        callback_id: CallbackId,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        if let Some(payload) = self.take_buffered_callback_notice(callback_id) {
            return Ok(payload);
        }

        let previous_timeout = self.socket_read_timeout()?;
        let deadline = Instant::now() + timeout;
        let result = self.wait_for_callback_notice_until_deadline(callback_id, timeout, deadline);
        let restore_result = self.set_socket_read_timeout(previous_timeout);

        match (result, restore_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) | (Err(_), Err(error)) => Err(error),
        }
    }

    fn wait_for_callback_notice_until_deadline(
        &mut self,
        callback_id: CallbackId,
        timeout: Duration,
        deadline: Instant,
    ) -> Result<Value, TransportError> {
        loop {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Err(TransportError::CallbackTimeout {
                    callback_id,
                    timeout,
                });
            };
            self.set_socket_read_timeout(Some(remaining))?;

            match self.read_inbound_for_callback(callback_id, timeout)? {
                JsonRpcInbound::Notice {
                    callback_id: actual,
                    payload,
                } if actual == callback_id => return Ok(payload),
                notice @ JsonRpcInbound::Notice { .. } => {
                    self.buffered_notices.push_back(notice);
                }
                response @ (JsonRpcInbound::Response { .. } | JsonRpcInbound::Error { .. }) => {
                    self.buffered_responses.push_back(response);
                }
            }
        }
    }

    fn take_buffered_response(&mut self, id: u64) -> Option<Result<Value, TransportError>> {
        let position = self.buffered_responses.iter().position(|inbound| {
            matches!(
                inbound,
                JsonRpcInbound::Response { id: actual, .. } | JsonRpcInbound::Error { id: actual, .. }
                    if *actual == id
            )
        })?;
        match self.buffered_responses.remove(position)? {
            JsonRpcInbound::Response { result, .. } => Some(Ok(result)),
            JsonRpcInbound::Error { id, error } => {
                Some(Err(TransportError::RpcError { id, error }))
            }
            JsonRpcInbound::Notice { .. } => None,
        }
    }

    fn take_buffered_callback_notice(&mut self, callback_id: CallbackId) -> Option<Value> {
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

    pub(crate) fn read_inbound_timeout(
        &mut self,
    ) -> Result<Option<JsonRpcInbound>, TransportError> {
        loop {
            match self.socket.read() {
                Ok(Message::Text(text)) => {
                    let value = serde_json::from_str(&text).map_err(TransportError::Json)?;
                    return parse_inbound(&value).map(Some);
                }
                Ok(Message::Close(_)) => return Err(TransportError::ConnectionClosed),
                Ok(Message::Binary(_))
                | Ok(Message::Ping(_))
                | Ok(Message::Pong(_))
                | Ok(Message::Frame(_)) => continue,
                Err(WebSocketError::Io(error))
                    if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
                {
                    return Ok(None);
                }
                Err(error) => return Err(TransportError::websocket(error)),
            }
        }
    }

    fn read_inbound_for_callback(
        &mut self,
        callback_id: CallbackId,
        timeout: Duration,
    ) -> Result<JsonRpcInbound, TransportError> {
        loop {
            match self.socket.read() {
                Ok(Message::Text(text)) => {
                    let value = serde_json::from_str(&text).map_err(TransportError::Json)?;
                    return parse_inbound(&value);
                }
                Ok(Message::Close(_)) => return Err(TransportError::ConnectionClosed),
                Ok(Message::Binary(_))
                | Ok(Message::Ping(_))
                | Ok(Message::Pong(_))
                | Ok(Message::Frame(_)) => continue,
                Err(WebSocketError::Io(error))
                    if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
                {
                    return Err(TransportError::CallbackTimeout {
                        callback_id,
                        timeout,
                    });
                }
                Err(error) => return Err(TransportError::websocket(error)),
            }
        }
    }

    pub(crate) fn socket_read_timeout(&self) -> Result<Option<Duration>, TransportError> {
        match self.socket.get_ref() {
            MaybeTlsStream::Plain(stream) => stream.read_timeout().map_err(TransportError::Io),
            MaybeTlsStream::NativeTls(stream) => {
                stream.get_ref().read_timeout().map_err(TransportError::Io)
            }
            _ => Err(TransportError::WebSocket(
                "unsupported websocket stream type for callback timeout".to_string(),
            )),
        }
    }

    pub(crate) fn set_socket_read_timeout(
        &self,
        timeout: Option<Duration>,
    ) -> Result<(), TransportError> {
        match self.socket.get_ref() {
            MaybeTlsStream::Plain(stream) => {
                stream.set_read_timeout(timeout).map_err(TransportError::Io)
            }
            MaybeTlsStream::NativeTls(stream) => stream
                .get_ref()
                .set_read_timeout(timeout)
                .map_err(TransportError::Io),
            _ => Err(TransportError::WebSocket(
                "unsupported websocket stream type for callback timeout".to_string(),
            )),
        }
    }
}

pub(crate) fn params_with_callback(
    callback_id: CallbackId,
    params_after_callback: Value,
) -> Result<Value, TransportError> {
    let mut params = params_after_callback
        .as_array()
        .cloned()
        .ok_or(TransportError::CallbackParamsNotArray)?;
    params.insert(0, json!(callback_id.as_u64()));
    Ok(Value::Array(params))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepends_graphene_callback_id_to_params_array() {
        assert_eq!(
            params_with_callback(CallbackId::new(7), json!([{"trx": true}])).unwrap(),
            json!([7, {"trx": true}])
        );
    }

    #[test]
    fn rejects_callback_params_that_are_not_an_array() {
        assert!(matches!(
            params_with_callback(CallbackId::new(7), json!({"trx": true})),
            Err(TransportError::CallbackParamsNotArray)
        ));
    }
}
