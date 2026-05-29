use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::Value;

use crate::{CallbackId, JsonRpcInbound, TransportError, WebSocketTransport};

const DISPATCH_READ_TIMEOUT: Duration = Duration::from_millis(50);

pub struct LiveTransport {
    commands: Sender<LiveCommand>,
    next_subscription_id: Arc<AtomicU64>,
    dispatcher: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub struct LiveTransportHandle {
    commands: Sender<LiveCommand>,
    next_subscription_id: Arc<AtomicU64>,
}

pub struct PendingResponse {
    receiver: Receiver<Result<Value, TransportError>>,
}

pub struct PendingCallbackNotice {
    callback_id: CallbackId,
    receiver: Receiver<Result<Value, TransportError>>,
}

pub struct LiveSubscription {
    callback_id: CallbackId,
    subscription_id: u64,
    receiver: Receiver<Value>,
    commands: Sender<LiveCommand>,
}

enum PendingCall {
    Response {
        response: Sender<Result<Value, TransportError>>,
    },
    CallbackAck {
        callback_id: CallbackId,
    },
}

struct PendingCallback {
    callback: Sender<Result<Value, TransportError>>,
}

enum LiveCommand {
    Call {
        api_id: u64,
        method: String,
        params: Value,
        response: Sender<Result<Value, TransportError>>,
    },
    CallWithCallback {
        api_id: u64,
        method: String,
        params_after_callback: Value,
        response: Sender<Result<CallbackId, TransportError>>,
        callback: Sender<Result<Value, TransportError>>,
    },
    SubscribeCallback {
        callback_id: CallbackId,
        subscription_id: u64,
        notices: Sender<Value>,
        response: Sender<Result<(), TransportError>>,
    },
    UnsubscribeCallback {
        callback_id: CallbackId,
        subscription_id: u64,
    },
    Shutdown,
}

impl LiveTransport {
    pub(crate) fn spawn(transport: WebSocketTransport) -> Result<Self, TransportError> {
        transport.set_socket_read_timeout(Some(DISPATCH_READ_TIMEOUT))?;
        let (commands, receiver) = mpsc::channel();
        let next_subscription_id = Arc::new(AtomicU64::new(1));
        let dispatcher = thread::spawn(move || run_dispatcher(transport, receiver));
        Ok(Self {
            commands,
            next_subscription_id,
            dispatcher: Some(dispatcher),
        })
    }

    pub fn handle(&self) -> LiveTransportHandle {
        LiveTransportHandle {
            commands: self.commands.clone(),
            next_subscription_id: self.next_subscription_id.clone(),
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

    pub fn call_with_callback(
        &self,
        api_id: u64,
        method: impl Into<String>,
        params_after_callback: Value,
    ) -> Result<PendingCallbackNotice, TransportError> {
        self.handle()
            .call_with_callback(api_id, method, params_after_callback)
    }

    pub fn subscribe_callback(
        &self,
        callback_id: CallbackId,
    ) -> Result<LiveSubscription, TransportError> {
        self.handle().subscribe_callback(callback_id)
    }
}

impl Drop for LiveTransport {
    fn drop(&mut self) {
        let _ = self.commands.send(LiveCommand::Shutdown);
        if let Some(dispatcher) = self.dispatcher.take() {
            let _ = dispatcher.join();
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
        let (response, receiver) = mpsc::channel();
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

    pub fn call_with_callback(
        &self,
        api_id: u64,
        method: impl Into<String>,
        params_after_callback: Value,
    ) -> Result<PendingCallbackNotice, TransportError> {
        let (callback_id_response, callback_id_receiver) = mpsc::channel();
        let (callback, receiver) = mpsc::channel();
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
            .recv()
            .map_err(|_| TransportError::DispatcherStopped)??;
        Ok(PendingCallbackNotice {
            callback_id,
            receiver,
        })
    }

    pub fn subscribe_callback(
        &self,
        callback_id: CallbackId,
    ) -> Result<LiveSubscription, TransportError> {
        let (notices, receiver) = mpsc::channel();
        let (response, response_receiver) = mpsc::channel();
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
            .recv()
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
    pub fn wait(self) -> Result<Value, TransportError> {
        self.receiver
            .recv()
            .map_err(|_| TransportError::DispatcherStopped)?
    }

    pub fn wait_timeout(self, timeout: Duration) -> Result<Value, TransportError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(TransportError::ResponseTimeout { timeout }),
            Err(RecvTimeoutError::Disconnected) => Err(TransportError::DispatcherStopped),
        }
    }
}

impl PendingCallbackNotice {
    pub fn callback_id(&self) -> CallbackId {
        self.callback_id
    }

    pub fn wait(self) -> Result<Value, TransportError> {
        self.receiver
            .recv()
            .map_err(|_| TransportError::DispatcherStopped)?
    }

    pub fn wait_timeout(self, timeout: Duration) -> Result<Value, TransportError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(TransportError::CallbackTimeout {
                callback_id: self.callback_id,
                timeout,
            }),
            Err(RecvTimeoutError::Disconnected) => Err(TransportError::DispatcherStopped),
        }
    }
}

impl LiveSubscription {
    pub fn callback_id(&self) -> CallbackId {
        self.callback_id
    }

    pub fn next(&self) -> Result<Value, TransportError> {
        self.receiver
            .recv()
            .map_err(|_| TransportError::DispatcherStopped)
    }

    pub fn next_timeout(&self, timeout: Duration) -> Result<Value, TransportError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(value) => Ok(value),
            Err(RecvTimeoutError::Timeout) => Err(TransportError::CallbackTimeout {
                callback_id: self.callback_id,
                timeout,
            }),
            Err(RecvTimeoutError::Disconnected) => Err(TransportError::DispatcherStopped),
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

fn run_dispatcher(mut transport: WebSocketTransport, commands: Receiver<LiveCommand>) {
    let mut pending_calls = HashMap::<u64, PendingCall>::new();
    let mut pending_callbacks = HashMap::<CallbackId, PendingCallback>::new();
    let mut subscriptions = HashMap::<CallbackId, Vec<(u64, Sender<Value>)>>::new();

    loop {
        while let Ok(command) = commands.try_recv() {
            match command {
                LiveCommand::Call {
                    api_id,
                    method,
                    params,
                    response,
                } => match transport.send_request(api_id, &method, params) {
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
                    match transport.send_callback_request(api_id, &method, params_after_callback) {
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
                    add_subscription(&mut subscriptions, callback_id, subscription_id, notices);
                    let _ = response.send(Ok(()));
                }
                LiveCommand::UnsubscribeCallback {
                    callback_id,
                    subscription_id,
                } => {
                    remove_subscription(&mut subscriptions, callback_id, subscription_id);
                }
                LiveCommand::Shutdown => return,
            }
        }

        match transport.read_inbound_timeout() {
            Ok(Some(inbound)) => {
                dispatch_inbound(
                    inbound,
                    &mut pending_calls,
                    &mut pending_callbacks,
                    &mut subscriptions,
                );
            }
            Ok(None) => {}
            Err(error) => {
                fail_all(error, &mut pending_calls, &mut pending_callbacks);
                return;
            }
        }
    }
}

fn add_subscription(
    subscriptions: &mut HashMap<CallbackId, Vec<(u64, Sender<Value>)>>,
    callback_id: CallbackId,
    subscription_id: u64,
    notices: Sender<Value>,
) {
    subscriptions
        .entry(callback_id)
        .or_default()
        .push((subscription_id, notices));
}

fn remove_subscription(
    subscriptions: &mut HashMap<CallbackId, Vec<(u64, Sender<Value>)>>,
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
    subscriptions: &mut HashMap<CallbackId, Vec<(u64, Sender<Value>)>>,
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
    use std::sync::mpsc::TryRecvError;

    fn response_pending() -> (PendingCall, Receiver<Result<Value, TransportError>>) {
        let (sender, receiver) = mpsc::channel();
        (PendingCall::Response { response: sender }, receiver)
    }

    fn callback_pending() -> (PendingCallback, Receiver<Result<Value, TransportError>>) {
        let (sender, receiver) = mpsc::channel();
        (PendingCallback { callback: sender }, receiver)
    }

    #[test]
    fn routes_responses_to_matching_pending_calls_out_of_order() {
        let (pending_one, receiver_one) = response_pending();
        let (pending_two, receiver_two) = response_pending();
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
        assert!(matches!(receiver_one.try_recv(), Err(TryRecvError::Empty)));
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
        let (pending_one, receiver_one) = callback_pending();
        let (pending_two, receiver_two) = callback_pending();
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
        assert!(matches!(receiver_one.try_recv(), Err(TryRecvError::Empty)));
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
        let (subscription_sender, subscription_receiver) = mpsc::channel();
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
        let (subscription_one, receiver_one) = mpsc::channel();
        let (subscription_two, receiver_two) = mpsc::channel();
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
        let (subscription_one, receiver_one) = mpsc::channel();
        let (subscription_two, receiver_two) = mpsc::channel();
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

        assert!(matches!(
            receiver_one.try_recv(),
            Err(TryRecvError::Empty | TryRecvError::Disconnected)
        ));
        assert_eq!(receiver_two.try_recv().unwrap(), json!("still-subscribed"));
        assert!(subscriptions.contains_key(&callback_id));
    }

    #[test]
    fn unsubscribe_last_subscription_removes_callback_entry() {
        let callback_id = CallbackId::new(42);
        let (subscription, _receiver) = mpsc::channel();
        let mut subscriptions = HashMap::new();
        add_subscription(&mut subscriptions, callback_id, 1, subscription);

        remove_subscription(&mut subscriptions, callback_id, 1);

        assert!(!subscriptions.contains_key(&callback_id));
    }

    #[test]
    fn closed_subscription_channel_does_not_block_other_subscribers() {
        let callback_id = CallbackId::new(42);
        let (closed_subscription, closed_receiver) = mpsc::channel();
        let (active_subscription, active_receiver) = mpsc::channel();
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
        let (pending_callback, callback_receiver) = callback_pending();
        let (subscription_sender, subscription_receiver) = mpsc::channel();
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
        assert!(matches!(
            subscription_receiver.try_recv(),
            Err(TryRecvError::Empty)
        ));
    }

    #[test]
    fn rpc_error_for_pending_callback_reaches_callback_receiver() {
        let callback_id = CallbackId::new(7);
        let (pending_callback, callback_receiver) = callback_pending();
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
        let (pending, receiver) = response_pending();
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

        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        assert!(pending_calls.contains_key(&1));
    }
}
