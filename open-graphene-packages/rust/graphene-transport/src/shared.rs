//! One connection to a node that many sessions use at once.
//!
//! A [`GrapheneSession`] with a connection of its own sends one request and reads until its
//! answer, so everyone who shares that session waits in line, one call at a time. Given to a
//! [`SharedConnection`], the socket is driven by the live dispatcher ([`LiveTransport`]) instead,
//! and the sessions made from it ([`SharedConnection::session`]) send their calls side by side:
//! a thousand resources of one deployment read and write the chain over one socket at once.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde_json::Value;

use crate::{
    ApiIds, CallbackId, DEFAULT_CALL_TIMEOUT, GrapheneSession, JsonRpcInbound, LiveTransport,
    PendingCallback, PendingCallbackNotice, TransportError, WebSocketTransport,
};

/// A connection whose calls the live dispatcher carries side by side, for every session made
/// from it.
pub struct SharedConnection {
    live: LiveTransport,
    url: String,
    api_ids: ApiIds,
    chain_id: String,
    /// Notices of the callback requests sent through sessions of this connection, by callback id,
    /// until a session takes its notice: a request and the wait for its notice may come from
    /// two sessions made one after the other.
    notices: Mutex<HashMap<CallbackId, PendingCallbackNotice>>,
}

impl SharedConnection {
    pub(crate) fn new(transport: WebSocketTransport, api_ids: ApiIds, chain_id: String) -> Self {
        let url = transport.url().to_owned();

        Self {
            live: LiveTransport::spawn(transport),
            url,
            api_ids,
            chain_id,
            notices: Mutex::new(HashMap::new()),
        }
    }

    /// A session whose calls go over this connection, side by side with every other session's.
    /// Making one costs nothing: no request reaches the node.
    pub fn session(self: &Arc<Self>) -> GrapheneSession {
        GrapheneSession::on_shared(self.clone())
    }

    /// The node this connection dialed, so that its owner can open a new one to the same node.
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn api_ids(&self) -> &ApiIds {
        &self.api_ids
    }

    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    /// Whether the connection has closed, as when the node went away: every call fails from
    /// then on, so its owner opens a new one.
    pub fn is_closed(&self) -> bool {
        self.live.is_stopped()
    }

    fn notices(&self) -> MutexGuard<'_, HashMap<CallbackId, PendingCallbackNotice>> {
        // A panic while the map was held leaves it as it was: every change is one insert or remove.
        self.notices
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn take_notice(
        &self,
        pending: PendingCallback,
    ) -> Result<PendingCallbackNotice, TransportError> {
        let callback_id = pending.callback_id();

        self.notices()
            .remove(&callback_id)
            .ok_or(TransportError::UnknownCallback { callback_id })
    }

    fn keep_notice(&self, notice: PendingCallbackNotice) {
        self.notices().insert(notice.callback_id(), notice);
    }
}

/// What a session talks to the node through: a connection of its own, or a shared one.
// A session holds its transport for its whole life and there is one per connection or per call,
// so the size of the variant of its own does not matter.
#[allow(clippy::large_enum_variant)]
pub(crate) enum SessionTransport {
    Own(WebSocketTransport),
    Shared(Arc<SharedConnection>),
}

impl SessionTransport {
    pub(crate) async fn call(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, TransportError> {
        match self {
            Self::Own(transport) => transport.call(api_id, method, params).await,
            Self::Shared(connection) => {
                let pending = connection.live.call(api_id, method, params)?;

                pending.wait_timeout(DEFAULT_CALL_TIMEOUT).await
            }
        }
    }

    /// The node's answer to a callback request, then its notice.
    pub(crate) async fn call_with_callback(
        &mut self,
        api_id: u64,
        method: &str,
        params_after_callback: Value,
    ) -> Result<Value, TransportError> {
        match self {
            Self::Own(transport) => {
                transport
                    .call_with_callback(api_id, method, params_after_callback)
                    .await
            }
            Self::Shared(connection) => {
                let mut notice = connection
                    .live
                    .call_with_callback(api_id, method, params_after_callback)
                    .await?;
                notice.accepted(DEFAULT_CALL_TIMEOUT).await?;

                notice.wait().await
            }
        }
    }

    pub(crate) async fn call_with_callback_timeout(
        &mut self,
        api_id: u64,
        method: &str,
        params_after_callback: Value,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        match self {
            Self::Own(transport) => {
                transport
                    .call_with_callback_timeout(api_id, method, params_after_callback, timeout)
                    .await
            }
            Self::Shared(connection) => {
                let mut notice = connection
                    .live
                    .call_with_callback(api_id, method, params_after_callback)
                    .await?;
                notice.accepted(DEFAULT_CALL_TIMEOUT).await?;

                notice.wait_timeout(timeout).await
            }
        }
    }

    pub(crate) async fn send_callback_request(
        &mut self,
        api_id: u64,
        method: &str,
        params_after_callback: Value,
    ) -> Result<PendingCallback, TransportError> {
        match self {
            Self::Own(transport) => {
                transport
                    .send_callback_request(api_id, method, params_after_callback)
                    .await
            }
            Self::Shared(connection) => {
                let notice = connection
                    .live
                    .call_with_callback(api_id, method, params_after_callback)
                    .await?;
                let pending = PendingCallback::new(notice.callback_id());
                connection.keep_notice(notice);

                Ok(pending)
            }
        }
    }

    /// The node's answer to a callback request: its acceptance, or its refusal.
    pub(crate) async fn wait_for_callback_response(
        &mut self,
        pending: PendingCallback,
    ) -> Result<Value, TransportError> {
        match self {
            Self::Own(transport) => transport.wait_for_callback_response(pending).await,
            Self::Shared(connection) => {
                let mut notice = connection.take_notice(pending)?;
                let answer = notice.accepted(DEFAULT_CALL_TIMEOUT).await;

                // A refused request never brings its notice; an accepted one waits for it here.
                if answer.is_ok() {
                    connection.keep_notice(notice);
                }

                answer
            }
        }
    }

    pub(crate) async fn poll_callback_notice(
        &mut self,
        pending: PendingCallback,
        timeout: Duration,
    ) -> Result<Option<Value>, TransportError> {
        match self {
            Self::Own(transport) => transport.poll_callback_notice(pending, timeout).await,
            Self::Shared(connection) => {
                let mut notice = connection.take_notice(pending)?;
                let polled = notice.poll(timeout).await;

                let not_yet = matches!(polled, Ok(None));
                if not_yet {
                    connection.keep_notice(notice);
                }

                polled
            }
        }
    }

    pub(crate) async fn wait_for_callback_response_and_notice_timeout(
        &mut self,
        pending: PendingCallback,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        match self {
            Self::Own(transport) => {
                transport
                    .wait_for_callback_response_and_notice_timeout(pending, timeout)
                    .await
            }
            Self::Shared(connection) => {
                let mut notice = connection.take_notice(pending)?;
                notice.accepted(DEFAULT_CALL_TIMEOUT).await?;

                notice.wait_timeout(timeout).await
            }
        }
    }

    /// The next notice of a subscription; a shared connection hands its notices to the
    /// subscriptions of its live transport instead.
    pub(crate) async fn next_notice(&mut self) -> Result<JsonRpcInbound, TransportError> {
        match self {
            Self::Own(transport) => transport.next_notice().await,
            Self::Shared(_) => Err(TransportError::NotOnSharedConnection {
                operation: "next_notice",
            }),
        }
    }
}
