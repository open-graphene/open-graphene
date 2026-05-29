use std::time::{Duration, Instant};

use graphene_chain_swaplock_bindings::generated::OperationHistoryObject;
use open_graphene_transport::{CallbackId, LiveSubscription, LiveTransportHandle};
use serde_json::json;

use crate::SwaplockApiError;
use crate::history::{
    ACCOUNT_HISTORY_METHOD, AccountHistoryPage, DEFAULT_ACCOUNT_HISTORY_LIMIT,
    DEFAULT_ACCOUNT_HISTORY_OFFSET, HISTORY_START_SENTINEL, HISTORY_STOP_SENTINEL,
    MAX_ACCOUNT_HISTORY_LIMIT, account_history_items_from_value, account_history_page_from_items,
    account_history_params,
};

use super::{LIVE_DATABASE_CALLBACK_ID, remaining_or_callback_timeout};

pub struct SwaplockLiveHistoryApi {
    pub(crate) live: LiveTransportHandle,
    pub(crate) database_api_id: u64,
    pub(crate) history_api_id: u64,
}

pub struct SwaplockLiveAccountHistoryRequest {
    live: LiveTransportHandle,
    database_api_id: u64,
    history_api_id: u64,
    account_name_or_id: String,
    limit: u32,
    offset: u32,
}

pub struct SwaplockLiveAccountHistoryByIdRequest {
    live: LiveTransportHandle,
    database_api_id: u64,
    history_api_id: u64,
    account_id: String,
    limit: u32,
    offset: u32,
}

pub struct SwaplockLiveAccountHistorySubscription {
    live: LiveTransportHandle,
    history_api_id: u64,
    account_name_or_id: String,
    initial: AccountHistoryPage,
    last_seen_operation_id: Option<String>,
    subscription: LiveSubscription,
}

impl SwaplockLiveHistoryApi {
    pub fn account_history(
        self,
        account_name_or_id: impl Into<String>,
    ) -> SwaplockLiveAccountHistoryRequest {
        SwaplockLiveAccountHistoryRequest {
            live: self.live,
            database_api_id: self.database_api_id,
            history_api_id: self.history_api_id,
            account_name_or_id: account_name_or_id.into(),
            limit: DEFAULT_ACCOUNT_HISTORY_LIMIT,
            offset: DEFAULT_ACCOUNT_HISTORY_OFFSET,
        }
    }

    pub fn account_history_by_id(
        self,
        account_id: impl Into<String>,
    ) -> SwaplockLiveAccountHistoryByIdRequest {
        SwaplockLiveAccountHistoryByIdRequest {
            live: self.live,
            database_api_id: self.database_api_id,
            history_api_id: self.history_api_id,
            account_id: account_id.into(),
            limit: DEFAULT_ACCOUNT_HISTORY_LIMIT,
            offset: DEFAULT_ACCOUNT_HISTORY_OFFSET,
        }
    }
}

impl SwaplockLiveAccountHistoryRequest {
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    pub fn subscribe(self) -> Result<SwaplockLiveAccountHistorySubscription, SwaplockApiError> {
        self.subscribe_timeout(Duration::from_secs(10))
    }

    pub fn subscribe_timeout(
        self,
        timeout: Duration,
    ) -> Result<SwaplockLiveAccountHistorySubscription, SwaplockApiError> {
        subscribe_live_account_history(
            self.live,
            self.database_api_id,
            self.history_api_id,
            self.account_name_or_id,
            self.limit,
            self.offset,
            timeout,
        )
    }
}

impl SwaplockLiveAccountHistoryByIdRequest {
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    pub fn subscribe(self) -> Result<SwaplockLiveAccountHistorySubscription, SwaplockApiError> {
        self.subscribe_timeout(Duration::from_secs(10))
    }

    pub fn subscribe_timeout(
        self,
        timeout: Duration,
    ) -> Result<SwaplockLiveAccountHistorySubscription, SwaplockApiError> {
        subscribe_live_account_history(
            self.live,
            self.database_api_id,
            self.history_api_id,
            self.account_id,
            self.limit,
            self.offset,
            timeout,
        )
    }
}

impl SwaplockLiveAccountHistorySubscription {
    pub fn initial(&self) -> &AccountHistoryPage {
        &self.initial
    }

    pub fn next_update(&mut self) -> Result<Vec<OperationHistoryObject>, SwaplockApiError> {
        loop {
            self.subscription.next()?;
            let updates = get_recent_live_account_history_since(
                &self.live,
                self.history_api_id,
                &self.account_name_or_id,
                self.last_seen_operation_id.as_deref(),
                None,
            )?;
            if !updates.is_empty() {
                self.last_seen_operation_id = updates.first().map(|item| item.id.0.clone());
                return Ok(updates);
            }
        }
    }

    pub fn next_update_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Vec<OperationHistoryObject>, SwaplockApiError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining =
                remaining_or_callback_timeout(deadline, self.subscription.callback_id(), timeout)?;
            self.subscription.next_timeout(remaining)?;
            let remaining =
                remaining_or_callback_timeout(deadline, self.subscription.callback_id(), timeout)?;
            let updates = get_recent_live_account_history_since(
                &self.live,
                self.history_api_id,
                &self.account_name_or_id,
                self.last_seen_operation_id.as_deref(),
                Some(remaining),
            )?;
            if !updates.is_empty() {
                self.last_seen_operation_id = updates.first().map(|item| item.id.0.clone());
                return Ok(updates);
            }
        }
    }
}

fn subscribe_live_account_history(
    live: LiveTransportHandle,
    database_api_id: u64,
    history_api_id: u64,
    account_name_or_id: String,
    limit: u32,
    offset: u32,
    timeout: Duration,
) -> Result<SwaplockLiveAccountHistorySubscription, SwaplockApiError> {
    let callback_id = CallbackId::new(LIVE_DATABASE_CALLBACK_ID);
    let subscription = live.subscribe_callback(callback_id)?;
    live.call(
        database_api_id,
        "set_subscribe_callback",
        json!([LIVE_DATABASE_CALLBACK_ID, false]),
    )?
    .wait_timeout(timeout)?;
    live.call(
        database_api_id,
        "get_full_accounts",
        json!([[account_name_or_id.clone()], true]),
    )?
    .wait_timeout(timeout)?;

    let initial = get_live_account_history_snapshot(
        &live,
        history_api_id,
        &account_name_or_id,
        limit,
        offset,
        Some(timeout),
    )?;

    Ok(SwaplockLiveAccountHistorySubscription {
        live,
        history_api_id,
        account_name_or_id,
        last_seen_operation_id: initial.newest_operation_id,
        initial: initial.page,
        subscription,
    })
}

fn get_live_account_history_snapshot(
    live: &LiveTransportHandle,
    history_api_id: u64,
    account_name_or_id: &str,
    limit: u32,
    offset: u32,
    timeout: Option<Duration>,
) -> Result<LiveAccountHistorySnapshot, SwaplockApiError> {
    let params = account_history_params(account_name_or_id, limit, offset)?;
    let pending = live.call(history_api_id, ACCOUNT_HISTORY_METHOD, params)?;
    let value = match timeout {
        Some(timeout) => pending.wait_timeout(timeout)?,
        None => pending.wait()?,
    };
    let raw_items = account_history_items_from_value(value)?;
    let newest_operation_id = raw_items.first().map(|item| item.id.0.clone());
    let page = account_history_page_from_items(raw_items, limit, offset);

    Ok(LiveAccountHistorySnapshot {
        page,
        newest_operation_id,
    })
}

fn get_recent_live_account_history_since(
    live: &LiveTransportHandle,
    history_api_id: u64,
    account_name_or_id: &str,
    last_seen_operation_id: Option<&str>,
    timeout: Option<Duration>,
) -> Result<Vec<OperationHistoryObject>, SwaplockApiError> {
    let stop = last_seen_operation_id.unwrap_or(HISTORY_STOP_SENTINEL);
    let pending = live.call(
        history_api_id,
        ACCOUNT_HISTORY_METHOD,
        json!([
            account_name_or_id,
            stop,
            MAX_ACCOUNT_HISTORY_LIMIT,
            HISTORY_START_SENTINEL
        ]),
    )?;
    let value = match timeout {
        Some(timeout) => pending.wait_timeout(timeout)?,
        None => pending.wait()?,
    };
    account_history_items_from_value(value)
}

struct LiveAccountHistorySnapshot {
    page: AccountHistoryPage,
    newest_operation_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn operation_history_fixture(id: &str) -> Value {
        json!({
            "id": id,
            "op": [0, {
                "fee": {"amount": 0, "asset_id": "1.3.0"},
                "from": "1.2.100",
                "to": "1.2.101",
                "amount": {"amount": 1, "asset_id": "1.3.0"},
                "memo": null,
                "extensions": []
            }],
            "result": [0, {}],
            "block_num": 1,
            "trx_in_block": 0,
            "op_in_trx": 0,
            "virtual_op": 0,
            "is_virtual": false,
            "block_time": "2026-01-01T00:00:00"
        })
    }

    #[test]
    fn live_history_parser_builds_endpoint_page_from_raw_items() {
        let raw_items = account_history_items_from_value(json!([
            operation_history_fixture("1.11.10"),
            operation_history_fixture("1.11.9"),
            operation_history_fixture("1.11.8")
        ]))
        .unwrap();
        let newest_operation_id = raw_items.first().map(|item| item.id.0.clone());
        let page = account_history_page_from_items(raw_items, 2, 0);

        assert_eq!(newest_operation_id.as_deref(), Some("1.11.10"));
        assert_eq!(page.items().len(), 2);
        assert_eq!(page.items()[0].id.0, "1.11.10");
        assert_eq!(page.items()[1].id.0, "1.11.9");
        assert_eq!(page.next_offset(), Some(2));
    }

    #[test]
    fn live_history_params_reuse_blocking_offset_wire_shape() {
        assert_eq!(
            account_history_params("1.2.100", 5, 10).unwrap(),
            json!(["1.2.100", "1.11.0", 16, "1.11.0"])
        );
    }
}
