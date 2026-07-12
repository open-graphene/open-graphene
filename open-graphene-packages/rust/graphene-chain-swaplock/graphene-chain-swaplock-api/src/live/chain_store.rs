//! A reactive object cache: subscribe to a set of object ids and keep their latest state.
//!
//! Seeds itself with `get_objects`, then a background worker drains the database subscription and
//! applies every pushed update to a shared map, so reads are always the freshest the node has sent.
//! This is the Rust take on bitsharesjs `ChainStore`, scoped to explicit object ids.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use open_graphene_transport::{CallbackId, LiveSubscription, LiveTransportHandle};
use serde_json::{Value, json};
use tokio::task::JoinHandle;

use super::LIVE_DATABASE_CALLBACK_ID;
use crate::SwaplockApiError;

/// How long the worker waits for a notice before checking whether it should stop.
const WORKER_TICK: Duration = Duration::from_millis(250);

/// A live, self-updating cache of chain objects keyed by id.
///
/// Build it from [`SwaplockLiveDatabaseApi::chain_store`](crate::SwaplockLiveDatabaseApi::chain_store).
/// Read the current state with [`get`](Self::get) or [`snapshot`](Self::snapshot); the values track
/// the node on their own. Dropping the store stops its worker.
pub struct ChainStore {
    objects: Arc<Mutex<HashMap<String, Value>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ChainStore {
    pub(super) async fn start(
        live: LiveTransportHandle,
        database_api_id: u64,
        ids: Vec<String>,
        timeout: Duration,
    ) -> Result<Self, SwaplockApiError> {
        let callback_id = CallbackId::new(LIVE_DATABASE_CALLBACK_ID);
        let subscription = live.subscribe_callback(callback_id).await?;
        live.call(
            database_api_id,
            "set_subscribe_callback",
            json!([LIVE_DATABASE_CALLBACK_ID, false]),
        )?
        .wait_timeout(timeout)
        .await?;

        // get_objects with subscribe=true seeds the cache and registers us for updates at once.
        let seed = live
            .call(database_api_id, "get_objects", json!([ids, true]))?
            .wait_timeout(timeout)
            .await?;

        let mut objects = HashMap::new();
        apply_notice(&mut objects, &seed);

        let objects = Arc::new(Mutex::new(objects));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = spawn_worker(subscription, Arc::clone(&objects), Arc::clone(&stop));

        Ok(Self {
            objects,
            stop,
            worker: Some(worker),
        })
    }

    /// The latest cached value for `id`, if it is in the store.
    pub fn get(&self, id: &str) -> Option<Value> {
        self.objects
            .lock()
            .expect("chain store mutex")
            .get(id)
            .cloned()
    }

    /// A copy of the whole cache, id to latest value.
    pub fn snapshot(&self) -> HashMap<String, Value> {
        self.objects.lock().expect("chain store mutex").clone()
    }

    /// How many objects the store currently holds.
    pub fn len(&self) -> usize {
        self.objects.lock().expect("chain store mutex").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for ChainStore {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            worker.abort();
        }
    }
}

fn spawn_worker(
    mut subscription: LiveSubscription,
    objects: Arc<Mutex<HashMap<String, Value>>>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while !stop.load(Ordering::Relaxed) {
            match subscription.next_timeout(WORKER_TICK).await {
                Ok(notice) => {
                    let mut guard = objects.lock().expect("chain store mutex");
                    apply_notice(&mut guard, &notice);
                }
                // A tick with no update: loop back and re-check the stop flag.
                Err(open_graphene_transport::TransportError::CallbackTimeout { .. }) => {}
                // Dispatcher gone (session dropped): nothing more will arrive.
                Err(_) => break,
            }
        }
    })
}

/// Fold every object carried by a `get_objects` result or a subscription notice into `objects`.
///
/// The node nests updates in arrays and may send a bare id string when an object is removed; we
/// update on each object that has a string `id`, and drop entries for removed ids.
fn apply_notice(objects: &mut HashMap<String, Value>, value: &Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                apply_notice(objects, item);
            }
        }
        Value::Object(map) => {
            if let Some(Value::String(id)) = map.get("id") {
                objects.insert(id.clone(), value.clone());
            }
        }
        Value::String(id) => {
            objects.remove(id);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_objects_and_removals_from_nested_notice() {
        let mut objects = HashMap::new();
        // get_objects shape: [ [ {obj}, {obj} ] ]
        apply_notice(
            &mut objects,
            &json!([[{"id": "2.1.0", "head": 1}, {"id": "1.3.0", "symbol": "BTS"}]]),
        );
        assert_eq!(objects.len(), 2);
        assert_eq!(objects["2.1.0"]["head"], json!(1));

        // a later notice updates one object...
        apply_notice(&mut objects, &json!([[{"id": "2.1.0", "head": 2}]]));
        assert_eq!(objects["2.1.0"]["head"], json!(2));

        // ...and a bare id string removes it.
        apply_notice(&mut objects, &json!([["1.3.0"]]));
        assert!(!objects.contains_key("1.3.0"));
        assert_eq!(objects.len(), 1);
    }
}
