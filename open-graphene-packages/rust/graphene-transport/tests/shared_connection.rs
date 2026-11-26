//! Sessions of one shared connection call a node side by side (`SharedConnection`), against a
//! node played here: it answers `slow` late and `fast` at once, and a broadcast with a callback
//! with its acceptance first and its notice later, or with a refusal.

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use open_graphene_transport::{GrapheneSession, TransportError};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tungstenite::Message;

const DATABASE: u64 = 2;
const NETWORK_BROADCAST: u64 = 3;
const SLOW_ANSWER: Duration = Duration::from_millis(400);
const NOTICE_AFTER: Duration = Duration::from_millis(100);

/// Starts the node and gives its address.
async fn node() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve(stream));
        }
    });

    format!("ws://{address}")
}

/// Answers one connection; late answers come from tasks of their own, through one writer.
async fn serve(stream: tokio::net::TcpStream) {
    let socket = tokio_tungstenite::accept_async(stream).await.unwrap();
    let (mut sink, mut source) = socket.split();

    let (outgoing, mut to_send) = mpsc::unbounded_channel::<Value>();
    tokio::spawn(async move {
        while let Some(message) = to_send.recv().await {
            if sink.send(Message::Text(message.to_string())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(Message::Text(text))) = source.next().await {
        let request: Value = serde_json::from_str(&text).unwrap();
        let id = request["id"].clone();
        let api = request["params"][0].as_u64().unwrap();
        let method = request["params"][1].as_str().unwrap().to_owned();
        let params = request["params"][2].clone();

        answer(&outgoing, id, api, &method, params);
    }
}

fn answer(
    outgoing: &mpsc::UnboundedSender<Value>,
    id: Value,
    api: u64,
    method: &str,
    params: Value,
) {
    let result = |value: Value| json!({ "id": id, "jsonrpc": "2.0", "result": value });
    let refusal =
        |message: &str| json!({ "id": id, "jsonrpc": "2.0", "error": { "message": message } });

    match (api, method) {
        (1, "login") => outgoing.send(result(json!(true))).unwrap(),
        (1, "database") => outgoing.send(result(json!(DATABASE))).unwrap(),
        (1, "network_broadcast") => outgoing.send(result(json!(NETWORK_BROADCAST))).unwrap(),
        (1, _) => outgoing.send(refusal("no such api")).unwrap(),
        (DATABASE, "get_chain_id") => outgoing.send(result(json!("test-chain"))).unwrap(),
        (DATABASE, "fast") => outgoing.send(result(json!("fast"))).unwrap(),
        (DATABASE, "slow") => {
            let outgoing = outgoing.clone();
            let late = result(json!("slow"));
            tokio::spawn(async move {
                tokio::time::sleep(SLOW_ANSWER).await;
                let _ = outgoing.send(late);
            });
        }
        (NETWORK_BROADCAST, "broadcast_transaction_with_callback") => {
            let callback_id = params[0].clone();
            if params[1] == json!("refuse") {
                outgoing
                    .send(refusal("the transaction is refused"))
                    .unwrap();
                return;
            }

            outgoing.send(result(Value::Null)).unwrap();

            let outgoing = outgoing.clone();
            let notice = json!({
                "method": "notice",
                "params": [callback_id, [{ "id": "trx", "block_num": 7 }]],
            });
            tokio::spawn(async move {
                tokio::time::sleep(NOTICE_AFTER).await;
                let _ = outgoing.send(notice);
            });
        }
        _ => outgoing.send(refusal("no such method")).unwrap(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fast_call_does_not_wait_for_a_slow_one_on_the_same_connection() {
    let url = node().await;
    let connection = GrapheneSession::connect(&url).await.unwrap().into_shared();
    let mut slow = connection.session();
    let mut fast = connection.session();

    let started = Instant::now();
    let slow_call = tokio::spawn(async move { slow.database_call("slow", json!([])).await });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let fast_answer = fast.database_call("fast", json!([])).await.unwrap();
    let fast_took = started.elapsed();
    let slow_answer = slow_call.await.unwrap().unwrap();

    assert_eq!(fast_answer, json!("fast"));
    assert_eq!(slow_answer, json!("slow"));
    assert!(
        fast_took < SLOW_ANSWER,
        "the fast call took {fast_took:?}, as if it waited for the slow one"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_notice_waits_for_whichever_session_of_the_connection_asks_for_it() {
    let url = node().await;
    let connection = GrapheneSession::connect(&url).await.unwrap().into_shared();
    let mut sender = connection.session();

    let pending = sender
        .network_broadcast_send_callback_request(
            "broadcast_transaction_with_callback",
            json!(["transaction"]),
        )
        .await
        .unwrap();
    sender
        .network_broadcast_wait_callback_response(pending)
        .await
        .unwrap();

    let mut waiter = connection.session();
    let mut notice = None;
    for _ in 0..20 {
        notice = waiter
            .network_broadcast_poll_callback_notice(pending, Duration::from_millis(50))
            .await
            .unwrap();
        if notice.is_some() {
            break;
        }
    }

    assert_eq!(notice, Some(json!([{ "id": "trx", "block_num": 7 }])));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_broadcast_is_told_at_once_and_brings_no_notice() {
    let url = node().await;
    let connection = GrapheneSession::connect(&url).await.unwrap().into_shared();
    let mut session = connection.session();

    let pending = session
        .network_broadcast_send_callback_request(
            "broadcast_transaction_with_callback",
            json!(["refuse"]),
        )
        .await
        .unwrap();
    let answer = session
        .network_broadcast_wait_callback_response(pending)
        .await;

    assert!(
        matches!(answer, Err(TransportError::RpcError { .. })),
        "{answer:?}"
    );

    let polled = session
        .network_broadcast_poll_callback_notice(pending, Duration::from_millis(10))
        .await;
    assert!(
        matches!(polled, Err(TransportError::UnknownCallback { .. })),
        "{polled:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_on_a_shared_connection_does_not_reconnect_by_itself() {
    let url = node().await;
    let connection = GrapheneSession::connect(&url).await.unwrap().into_shared();
    let mut session = connection.session();

    let reconnected = session.reconnect().await;

    assert!(
        matches!(
            reconnected,
            Err(TransportError::NotOnSharedConnection {
                operation: "reconnect"
            })
        ),
        "{reconnected:?}"
    );
    assert!(!connection.is_closed());
}
