use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::{CallbackId, JsonRpcInbound, TransportError, WebSocketTransport};

/// How often the dispatcher pings the node so NAT/idle timeouts cannot
/// silently kill an otherwise quiet connection.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);

/// Subscription callback ids live above this base so they can never collide
/// with the transport's request/callback-id counter (which starts at 1 and
/// grows per request). A one-shot broadcast callback stealing a subscription
/// notice with the same numeric id was a silent-corruption bug.
pub const SUBSCRIPTION_CALLBACK_ID_BASE: u64 = 1 << 32;

pub struct LiveTransport {
    commands: UnboundedSender<LiveCommand>,
    next_subscription_id: Arc<AtomicU64>,
    next_callback_id: Arc<AtomicU64>,
    dispatcher: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub struct LiveTransportHandle {
    commands: UnboundedSender<LiveCommand>,
    next_subscription_id: Arc<AtomicU64>,
    next_callback_id: Arc<AtomicU64>,
}

pub struct PendingResponse {
    receiver: oneshot::Receiver<Result<Value, TransportError>>,
}

pub struct PendingCallbackNotice {
    callback_id: CallbackId,
    receiver: oneshot::Receiver<Result<Value, TransportError>>,
}

pub struct LiveSubscription {
    callback_id: CallbackId,
    subscription_id: u64,
    receiver: UnboundedReceiver<Value>,
    commands: UnboundedSender<LiveCommand>,
}

enum PendingCall {
    Response {
        response: oneshot::Sender<Result<Value, TransportError>>,
    },
    CallbackAck {
        callback_id: CallbackId,
    },
}

struct PendingCallback {
    callback: oneshot::Sender<Result<Value, TransportError>>,
}

enum LiveCommand {
    Call {
        api_id: u64,
        method: String,
        params: Value,
        response: oneshot::Sender<Result<Value, TransportError>>,
    },
    CallWithCallback {
        api_id: u64,
        method: String,
        params_after_callback: Value,
        response: oneshot::Sender<Result<CallbackId, TransportError>>,
        callback: oneshot::Sender<Result<Value, TransportError>>,
    },
    SubscribeCallback {
        callback_id: CallbackId,
        subscription_id: u64,
        notices: UnboundedSender<Value>,
        response: oneshot::Sender<Result<(), TransportError>>,
    },
    UnsubscribeCallback {
        callback_id: CallbackId,
        subscription_id: u64,
    },
    Shutdown,
}

impl LiveTransport {
    pub(crate) fn spawn(transport: WebSocketTransport) -> Result<Self, TransportError> {
        let (commands, receiver) = unbounded_channel();
        let next_subscription_id = Arc::new(AtomicU64::new(1));
        let next_callback_id = Arc::new(AtomicU64::new(SUBSCRIPTION_CALLBACK_ID_BASE));
        let dispatcher = tokio::spawn(run_dispatcher(transport, receiver));
        Ok(Self {
            commands,
            next_subscription_id,
            next_callback_id,
            dispatcher: Some(dispatcher),
        })
    }

    pub fn handle(&self) -> LiveTransportHandle {
        LiveTransportHandle {
            commands: self.commands.clone(),
            next_subscription_id: self.next_subscription_id.clone(),
            next_callback_id: self.next_callback_id.clone(),
        }
    }

    pub fn call(
        &self,
        api_id: u64,
        method: impl Into<String>,
        params: Value,
    ) -> Result<PendingResponse, TransportError> {
        self.handle().call(api_id, method, params)
    }

    pub async fn call_with_callback(
        &self,
        api_id: u64,
        method: impl Into<String>,
        params_after_callback: Value,
    ) -> Result<PendingCallbackNotice, TransportError> {
        self.handle()
            .call_with_callback(api_id, method, params_after_callback)
            .await
    }

    pub async fn subscribe_callback(
        &self,
        callback_id: CallbackId,
    ) -> Result<LiveSubscription, TransportError> {
        self.handle().subscribe_callback(callback_id).await
    }
}

impl Drop for LiveTransport {
    fn drop(&mut self) {
        let _ = self.commands.send(LiveCommand::Shutdown);
        if let Some(dispatcher) = self.dispatcher.take() {
            dispatcher.abort();
        }
    }
}

impl LiveTransportHandle {
    pub fn call(
        &self,
        api_id: u64,
        method: impl Into<String>,
        params: Value,
    ) -> Result<PendingResponse, TransportError> {
        let (response, receiver) = oneshot::channel();
        self.commands
            .send(LiveCommand::Call {
                api_id,
                method: method.into(),
                params,
                response,
            })
            .map_err(|_| TransportError::DispatcherStopped)?;
        Ok(PendingResponse { receiver })
    }

    pub async fn call_with_callback(
        &self,
        api_id: u64,
        method: impl Into<String>,
        params_after_callback: Value,
    ) -> Result<PendingCallbackNotice, TransportError> {
        let (callback_id_response, callback_id_receiver) = oneshot::channel();
        let (callback, receiver) = oneshot::channel();
        self.commands
            .send(LiveCommand::CallWithCallback {
                api_id,
                method: method.into(),
                params_after_callback,
                response: callback_id_response,
                callback,
            })
            .map_err(|_| TransportError::DispatcherStopped)?;
        let callback_id = callback_id_receiver
            .await
            .map_err(|_| TransportError::DispatcherStopped)??;
        Ok(PendingCallbackNotice {
            callback_id,
            receiver,
        })
    }

    /// Allocate a fresh subscription callback id from the reserved range.
    /// Use this id both in the chain RPC that registers the callback (e.g.
    /// `set_subscribe_callback` params) and in [`Self::subscribe_callback`].
    pub fn allocate_callback_id(&self) -> CallbackId {
        CallbackId::new(self.next_callback_id.fetch_add(1, Ordering::Relaxed))
    }

    pub async fn subscribe_callback(
        &self,
        callback_id: CallbackId,
    ) -> Result<LiveSubscription, TransportError> {
        if callback_id.as_u64() < SUBSCRIPTION_CALLBACK_ID_BASE {
            return Err(TransportError::SubscriptionCallbackIdReserved {
                callback_id,
                base: SUBSCRIPTION_CALLBACK_ID_BASE,
            });
        }
        let (notices, receiver) = unbounded_channel();
        let (response, response_receiver) = oneshot::channel();
        let subscription_id = self.next_subscription_id.fetch_add(1, Ordering::Relaxed);
        self.commands
            .send(LiveCommand::SubscribeCallback {
                callback_id,
                subscription_id,
                notices,
                response,
            })
            .map_err(|_| TransportError::DispatcherStopped)?;
        response_receiver
            .await
            .map_err(|_| TransportError::DispatcherStopped)??;
        Ok(LiveSubscription {
            callback_id,
            subscription_id,
            receiver,
            commands: self.commands.clone(),
        })
    }
}

impl PendingResponse {
    pub async fn wait(self) -> Result<Value, TransportError> {
        self.receiver
            .await
            .map_err(|_| TransportError::DispatcherStopped)?
    }

    pub async fn wait_timeout(self, timeout: Duration) -> Result<Value, TransportError> {
        match tokio::time::timeout(timeout, self.receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(TransportError::DispatcherStopped),
            Err(_) => Err(TransportError::ResponseTimeout { timeout }),
        }
    }
}

impl PendingCallbackNotice {
    pub fn callback_id(&self) -> CallbackId {
        self.callback_id
    }

    pub async fn wait(self) -> Result<Value, TransportError> {
        self.receiver
            .await
            .map_err(|_| TransportError::DispatcherStopped)?
    }

    pub async fn wait_timeout(self, timeout: Duration) -> Result<Value, TransportError> {
        let callback_id = self.callback_id;
        match tokio::time::timeout(timeout, self.receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(TransportError::DispatcherStopped),
            Err(_) => Err(TransportError::CallbackTimeout {
                callback_id,
                timeout,
            }),
        }
    }
}

impl LiveSubscription {
    pub fn callback_id(&self) -> CallbackId {
        self.callback_id
    }

    pub async fn next(&mut self) -> Result<Value, TransportError> {
        self.receiver
            .recv()
            .await
            .ok_or(TransportError::DispatcherStopped)
    }

    pub async fn next_timeout(&mut self, timeout: Duration) -> Result<Value, TransportError> {
        match tokio::time::timeout(timeout, self.receiver.recv()).await {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(TransportError::DispatcherStopped),
            Err(_) => Err(TransportError::CallbackTimeout {
                callback_id: self.callback_id,
                timeout,
            }),
        }
    }
}

impl Drop for LiveSubscription {
    fn drop(&mut self) {
        let _ = self.commands.send(LiveCommand::UnsubscribeCallback {
            callback_id: self.callback_id,
            subscription_id: self.subscription_id,
        });
    }
}

enum DispatcherEvent {
    Command(Option<LiveCommand>),
    Inbound(Result<JsonRpcInbound, TransportError>),
    KeepaliveTick,
}

async fn run_dispatcher(
    mut transport: WebSocketTransport,
    mut commands: UnboundedReceiver<LiveCommand>,
) {
    let mut pending_calls = HashMap::<u64, PendingCall>::new();
    let mut pending_callbacks = HashMap::<CallbackId, PendingCallback>::new();
    let mut subscriptions = HashMap::<CallbackId, Vec<(u64, UnboundedSender<Value>)>>::new();
    let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    keepalive.reset();

    loop {
        // The select! resolves to an event first so the arms below can use
        // `transport` mutably without fighting the borrow held by the losing
        // futures. Outgoing commands are handled the moment they arrive —
        // no polling interval sits between a caller and the socket.
        let event = tokio::select! {
            command = commands.recv() => DispatcherEvent::Command(command),
            inbound = transport.read_inbound() => DispatcherEvent::Inbound(inbound),
            _ = keepalive.tick() => DispatcherEvent::KeepaliveTick,
        };

        match event {
            DispatcherEvent::Command(None)
            | DispatcherEvent::Command(Some(LiveCommand::Shutdown)) => {
                return;
            }
            DispatcherEvent::Command(Some(command)) => {
                handle_command(
                    command,
                    &mut transport,
                    &mut pending_calls,
                    &mut pending_callbacks,
                    &mut subscriptions,
                )
                .await;
            }
            DispatcherEvent::Inbound(Ok(inbound)) => {
                dispatch_inbound(
                    inbound,
                    &mut pending_calls,
                    &mut pending_callbacks,
                    &mut subscriptions,
                );
            }
            // One garbled frame must not kill every pending call and
            // subscription on a healthy connection: skip it.
            DispatcherEvent::Inbound(Err(error)) if error.is_malformed_frame() => {}
            DispatcherEvent::Inbound(Err(error)) => {
                fail_all(error, &mut pending_calls, &mut pending_callbacks);
                return;
            }
            DispatcherEvent::KeepaliveTick => {
                if let Err(error) = transport.send_ping().await {
                    fail_all(error, &mut pending_calls, &mut pending_callbacks);
                    return;
                }
            }
        }
    }
}

async fn handle_command(
    command: LiveCommand,
    transport: &mut WebSocketTransport,
    pending_calls: &mut HashMap<u64, PendingCall>,
    pending_callbacks: &mut HashMap<CallbackId, PendingCallback>,
    subscriptions: &mut HashMap<CallbackId, Vec<(u64, UnboundedSender<Value>)>>,
) {
    match command {
        LiveCommand::Call {
            api_id,
            method,
            params,
            response,
        } => match transport.send_request(api_id, &method, params).await {
            Ok(id) => {
                pending_calls.insert(id, PendingCall::Response { response });
            }
            Err(error) => {
                let _ = response.send(Err(error));
            }
        },
        LiveCommand::CallWithCallback {
            api_id,
            method,
            params_after_callback,
            response,
            callback,
        } => {
            match transport
                .send_callback_request(api_id, &method, params_after_callback)
                .await
            {
                Ok(pending) => {
                    let callback_id = pending.callback_id();
                    let _ = response.send(Ok(callback_id));
                    pending_calls.insert(
                        callback_id.as_u64(),
                        PendingCall::CallbackAck { callback_id },
                    );
                    pending_callbacks.insert(callback_id, PendingCallback { callback });
                }
                Err(error) => {
                    let _ = response.send(Err(error));
                }
            }
        }
        LiveCommand::SubscribeCallback {
            callback_id,
            subscription_id,
            notices,
            response,
        } => {
            add_subscription(subscriptions, callback_id, subscription_id, notices);
            let _ = response.send(Ok(()));
        }
        LiveCommand::UnsubscribeCallback {
            callback_id,
            subscription_id,
        } => {
            remove_subscription(subscriptions, callback_id, subscription_id);
        }
        LiveCommand::Shutdown => {}
    }
}

fn add_subscription(
    subscriptions: &mut HashMap<CallbackId, Vec<(u64, UnboundedSender<Value>)>>,
    callback_id: CallbackId,
    subscription_id: u64,
    notices: UnboundedSender<Value>,
) {
    subscriptions
        .entry(callback_id)
        .or_default()
        .push((subscription_id, notices));
}

fn remove_subscription(
    subscriptions: &mut HashMap<CallbackId, Vec<(u64, UnboundedSender<Value>)>>,
    callback_id: CallbackId,
    subscription_id: u64,
) {
    if let Some(callback_subscriptions) = subscriptions.get_mut(&callback_id) {
        callback_subscriptions.retain(|(id, _)| *id != subscription_id);
        if callback_subscriptions.is_empty() {
            subscriptions.remove(&callback_id);
        }
    }
}

fn dispatch_inbound(
    inbound: JsonRpcInbound,
    pending_calls: &mut HashMap<u64, PendingCall>,
    pending_callbacks: &mut HashMap<CallbackId, PendingCallback>,
    subscriptions: &mut HashMap<CallbackId, Vec<(u64, UnboundedSender<Value>)>>,
) {
    match inbound {
        JsonRpcInbound::Response { id, result } => {
            if let Some(PendingCall::Response { response }) = pending_calls.remove(&id) {
                let _ = response.send(Ok(result));
            }
        }
        JsonRpcInbound::Error { id, error } => {
            if let Some(pending) = pending_calls.remove(&id) {
                let error = TransportError::RpcError { id, error };
                match pending {
                    PendingCall::Response { response } => {
                        let _ = response.send(Err(error));
                    }
                    PendingCall::CallbackAck { callback_id } => {
                        if let Some(pending_callback) = pending_callbacks.remove(&callback_id) {
                            let _ = pending_callback.callback.send(Err(error));
                        }
                    }
                }
            }
        }
        JsonRpcInbound::Notice {
            callback_id,
            payload,
        } => {
            if let Some(pending) = pending_callbacks.remove(&callback_id) {
                let _ = pending.callback.send(Ok(payload));
            } else if let Some(callback_subscriptions) = subscriptions.get_mut(&callback_id) {
                callback_subscriptions
                    .retain(|(_, subscription)| subscription.send(payload.clone()).is_ok());
                if callback_subscriptions.is_empty() {
                    subscriptions.remove(&callback_id);
                }
            }
        }
    }
}

fn fail_all(
    error: TransportError,
    pending_calls: &mut HashMap<u64, PendingCall>,
    pending_callbacks: &mut HashMap<CallbackId, PendingCallback>,
) {
    let message = error.to_string();
    for (_, pending) in pending_calls.drain() {
        if let PendingCall::Response { response } = pending {
            let _ = response.send(Err(TransportError::WebSocket(message.clone())));
        }
    }
    for (_, pending) in pending_callbacks.drain() {
        let _ = pending
            .callback
            .send(Err(TransportError::WebSocket(message.clone())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn response_pending() -> (
        PendingCall,
        oneshot::Receiver<Result<Value, TransportError>>,
    ) {
        let (sender, receiver) = oneshot::channel();
        (PendingCall::Response { response: sender }, receiver)
    }

    fn callback_pending() -> (
        PendingCallback,
        oneshot::Receiver<Result<Value, TransportError>>,
    ) {
        let (sender, receiver) = oneshot::channel();
        (PendingCallback { callback: sender }, receiver)
    }

    #[test]
    fn routes_responses_to_matching_pending_calls_out_of_order() {
        let (pending_one, mut receiver_one) = response_pending();
        let (pending_two, mut receiver_two) = response_pending();
        let mut pending_calls = HashMap::from([(1, pending_one), (2, pending_two)]);
        let mut pending_callbacks = HashMap::new();
        let mut subscriptions = HashMap::new();

        dispatch_inbound(
            JsonRpcInbound::Response {
                id: 2,
                result: json!("second"),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert_eq!(receiver_two.try_recv().unwrap().unwrap(), json!("second"));
        assert!(receiver_one.try_recv().is_err());
        assert!(pending_calls.contains_key(&1));
        assert!(!pending_calls.contains_key(&2));

        dispatch_inbound(
            JsonRpcInbound::Response {
                id: 1,
                result: json!("first"),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert_eq!(receiver_one.try_recv().unwrap().unwrap(), json!("first"));
        assert!(pending_calls.is_empty());
    }

    #[test]
    fn routes_callback_notices_to_matching_pending_callbacks_out_of_order() {
        let callback_one = CallbackId::new(11);
        let callback_two = CallbackId::new(12);
        let (pending_one, mut receiver_one) = callback_pending();
        let (pending_two, mut receiver_two) = callback_pending();
        let mut pending_calls = HashMap::new();
        let mut pending_callbacks =
            HashMap::from([(callback_one, pending_one), (callback_two, pending_two)]);
        let mut subscriptions = HashMap::new();

        dispatch_inbound(
            JsonRpcInbound::Notice {
                callback_id: callback_two,
                payload: json!({"callback": 2}),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert_eq!(
            receiver_two.try_recv().unwrap().unwrap(),
            json!({"callback": 2})
        );
        assert!(receiver_one.try_recv().is_err());
        assert!(pending_callbacks.contains_key(&callback_one));
        assert!(!pending_callbacks.contains_key(&callback_two));

        dispatch_inbound(
            JsonRpcInbound::Notice {
                callback_id: callback_one,
                payload: json!({"callback": 1}),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert_eq!(
            receiver_one.try_recv().unwrap().unwrap(),
            json!({"callback": 1})
        );
        assert!(pending_callbacks.is_empty());
    }

    #[test]
    fn routes_subscription_notice_when_no_pending_callback_matches() {
        let callback_id = CallbackId::new(42);
        let (subscription_sender, mut subscription_receiver) = unbounded_channel();
        let mut pending_calls = HashMap::new();
        let mut pending_callbacks = HashMap::new();
        let mut subscriptions = HashMap::from([(callback_id, vec![(1, subscription_sender)])]);

        dispatch_inbound(
            JsonRpcInbound::Notice {
                callback_id,
                payload: json!([[{"id": "2.1.0"}]]),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert_eq!(
            subscription_receiver.try_recv().unwrap(),
            json!([[{"id": "2.1.0"}]])
        );
    }

    #[test]
    fn multicasts_subscription_notice_to_all_matching_subscribers() {
        let callback_id = CallbackId::new(42);
        let (subscription_one, mut receiver_one) = unbounded_channel();
        let (subscription_two, mut receiver_two) = unbounded_channel();
        let mut pending_calls = HashMap::new();
        let mut pending_callbacks = HashMap::new();
        let mut subscriptions = HashMap::from([(
            callback_id,
            vec![(1, subscription_one), (2, subscription_two)],
        )]);

        dispatch_inbound(
            JsonRpcInbound::Notice {
                callback_id,
                payload: json!([[{"id": "2.1.0"}]]),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert_eq!(receiver_one.try_recv().unwrap(), json!([[{"id": "2.1.0"}]]));
        assert_eq!(receiver_two.try_recv().unwrap(), json!([[{"id": "2.1.0"}]]));
    }

    #[test]
    fn unsubscribe_removes_only_the_matching_subscription() {
        let callback_id = CallbackId::new(42);
        let (subscription_one, mut receiver_one) = unbounded_channel();
        let (subscription_two, mut receiver_two) = unbounded_channel();
        let mut subscriptions = HashMap::new();
        add_subscription(&mut subscriptions, callback_id, 1, subscription_one);
        add_subscription(&mut subscriptions, callback_id, 2, subscription_two);

        remove_subscription(&mut subscriptions, callback_id, 1);

        let mut pending_calls = HashMap::new();
        let mut pending_callbacks = HashMap::new();
        dispatch_inbound(
            JsonRpcInbound::Notice {
                callback_id,
                payload: json!("still-subscribed"),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert!(receiver_one.try_recv().is_err());
        assert_eq!(receiver_two.try_recv().unwrap(), json!("still-subscribed"));
        assert!(subscriptions.contains_key(&callback_id));
    }

    #[test]
    fn unsubscribe_last_subscription_removes_callback_entry() {
        let callback_id = CallbackId::new(42);
        let (subscription, _receiver) = unbounded_channel();
        let mut subscriptions = HashMap::new();
        add_subscription(&mut subscriptions, callback_id, 1, subscription);

        remove_subscription(&mut subscriptions, callback_id, 1);

        assert!(!subscriptions.contains_key(&callback_id));
    }

    #[test]
    fn closed_subscription_channel_does_not_block_other_subscribers() {
        let callback_id = CallbackId::new(42);
        let (closed_subscription, closed_receiver) = unbounded_channel::<Value>();
        let (active_subscription, mut active_receiver) = unbounded_channel();
        drop(closed_receiver);
        let mut subscriptions = HashMap::from([(
            callback_id,
            vec![(1, closed_subscription), (2, active_subscription)],
        )]);
        let mut pending_calls = HashMap::new();
        let mut pending_callbacks = HashMap::new();

        dispatch_inbound(
            JsonRpcInbound::Notice {
                callback_id,
                payload: json!("active-only"),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert_eq!(active_receiver.try_recv().unwrap(), json!("active-only"));
        assert_eq!(subscriptions.get(&callback_id).unwrap().len(), 1);
        assert_eq!(subscriptions.get(&callback_id).unwrap()[0].0, 2);
    }

    #[test]
    fn pending_callback_takes_precedence_over_subscription_with_same_id() {
        let callback_id = CallbackId::new(42);
        let (pending_callback, mut callback_receiver) = callback_pending();
        let (subscription_sender, mut subscription_receiver) = unbounded_channel();
        let mut pending_calls = HashMap::new();
        let mut pending_callbacks = HashMap::from([(callback_id, pending_callback)]);
        let mut subscriptions = HashMap::from([(callback_id, vec![(1, subscription_sender)])]);

        dispatch_inbound(
            JsonRpcInbound::Notice {
                callback_id,
                payload: json!("pending-callback"),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert_eq!(
            callback_receiver.try_recv().unwrap().unwrap(),
            json!("pending-callback")
        );
        assert!(subscription_receiver.try_recv().is_err());
    }

    #[test]
    fn rpc_error_for_pending_callback_reaches_callback_receiver() {
        let callback_id = CallbackId::new(7);
        let (pending_callback, mut callback_receiver) = callback_pending();
        let mut pending_calls = HashMap::from([(
            callback_id.as_u64(),
            PendingCall::CallbackAck { callback_id },
        )]);
        let mut pending_callbacks = HashMap::from([(callback_id, pending_callback)]);
        let mut subscriptions = HashMap::new();

        dispatch_inbound(
            JsonRpcInbound::Error {
                id: callback_id.as_u64(),
                error: json!({"code": 3030001, "message": "duplicate transaction"}),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        let error = callback_receiver.try_recv().unwrap().unwrap_err();
        assert!(matches!(error, TransportError::RpcError { id: 7, .. }));
        assert!(pending_calls.is_empty());
        assert!(pending_callbacks.is_empty());
    }

    #[test]
    fn unknown_inbound_is_ignored_without_disturbing_pending_work() {
        let (pending, mut receiver) = response_pending();
        let mut pending_calls = HashMap::from([(1, pending)]);
        let mut pending_callbacks = HashMap::new();
        let mut subscriptions = HashMap::new();

        dispatch_inbound(
            JsonRpcInbound::Response {
                id: 999,
                result: json!("unknown"),
            },
            &mut pending_calls,
            &mut pending_callbacks,
            &mut subscriptions,
        );

        assert!(receiver.try_recv().is_err());
        assert!(pending_calls.contains_key(&1));
    }
}
