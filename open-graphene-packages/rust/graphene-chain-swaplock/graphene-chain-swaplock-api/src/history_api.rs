use graphene_chain_swaplock_bindings::generated::OperationHistoryObject;
use open_graphene_transport::{GrapheneSession, JsonRpcInbound};
use serde_json::{Value, json};

use crate::SwaplockApiError;

pub const DEFAULT_ACCOUNT_HISTORY_LIMIT: u32 = 20;
pub const DEFAULT_ACCOUNT_HISTORY_OFFSET: u32 = 0;
pub const MAX_ACCOUNT_HISTORY_LIMIT: u32 = 98;

const ACCOUNT_HISTORY_METHOD: &str = "get_account_history";
const HISTORY_START_SENTINEL: &str = "1.11.0";
const HISTORY_STOP_SENTINEL: &str = "1.11.0";
const ACCOUNT_HISTORY_CALLBACK_ID: u64 = 6;

pub struct HistoryApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

pub struct AccountHistoryRequest<'session> {
    session: &'session mut GrapheneSession,
    account_name_or_id: String,
    limit: u32,
    offset: u32,
}

pub struct AccountHistoryByIdRequest<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    limit: u32,
    offset: u32,
}

pub struct AccountHistorySubscription<'session> {
    session: &'session mut GrapheneSession,
    account_name_or_id: String,
    initial: AccountHistoryPage,
    last_seen_operation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccountHistoryPage {
    items: Vec<OperationHistoryObject>,
    limit: u32,
    offset: u32,
    next_offset: Option<u32>,
}

struct AccountHistorySnapshot {
    page: AccountHistoryPage,
    newest_operation_id: Option<String>,
}

impl AccountHistoryPage {
    pub fn items(&self) -> &[OperationHistoryObject] {
        &self.items
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }

    pub fn has_more(&self) -> bool {
        self.next_offset.is_some()
    }

    pub fn into_items(self) -> Vec<OperationHistoryObject> {
        self.items
    }
}

impl<'session> HistoryApi<'session> {
    pub fn account_history<S>(self, account_name_or_id: S) -> AccountHistoryRequest<'session>
    where
        S: Into<String>,
    {
        AccountHistoryRequest {
            session: self.session,
            account_name_or_id: account_name_or_id.into(),
            limit: DEFAULT_ACCOUNT_HISTORY_LIMIT,
            offset: DEFAULT_ACCOUNT_HISTORY_OFFSET,
        }
    }

    pub fn account_history_by_id<S>(self, account_id: S) -> AccountHistoryByIdRequest<'session>
    where
        S: Into<String>,
    {
        AccountHistoryByIdRequest {
            session: self.session,
            account_id: account_id.into(),
            limit: DEFAULT_ACCOUNT_HISTORY_LIMIT,
            offset: DEFAULT_ACCOUNT_HISTORY_OFFSET,
        }
    }
}

impl<'session> AccountHistoryRequest<'session> {
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    pub async fn get(self) -> Result<AccountHistoryPage, SwaplockApiError> {
        Ok(get_account_history_snapshot(
            self.session,
            &self.account_name_or_id,
            self.limit,
            self.offset,
        )
        .await?
        .page)
    }

    pub async fn subscribe(self) -> Result<AccountHistorySubscription<'session>, SwaplockApiError> {
        subscribe_account_history(
            self.session,
            self.account_name_or_id,
            self.limit,
            self.offset,
        )
        .await
    }
}

impl<'session> AccountHistoryByIdRequest<'session> {
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    pub async fn get(self) -> Result<AccountHistoryPage, SwaplockApiError> {
        Ok(
            get_account_history_snapshot(self.session, &self.account_id, self.limit, self.offset)
                .await?
                .page,
        )
    }

    pub async fn subscribe(self) -> Result<AccountHistorySubscription<'session>, SwaplockApiError> {
        subscribe_account_history(self.session, self.account_id, self.limit, self.offset).await
    }
}

impl AccountHistorySubscription<'_> {
    pub fn initial(&self) -> &AccountHistoryPage {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<Vec<OperationHistoryObject>, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice()?;
            let JsonRpcInbound::Notice { callback_id, .. } = notice else {
                continue;
            };
            if callback_id != ACCOUNT_HISTORY_CALLBACK_ID {
                continue;
            }

            let updates = get_recent_account_history_since(
                self.session,
                &self.account_name_or_id,
                self.last_seen_operation_id.as_deref(),
            )
            .await?;
            if !updates.is_empty() {
                self.last_seen_operation_id = updates.first().map(|item| item.id.0.clone());
                return Ok(updates);
            }
        }
    }
}

async fn subscribe_account_history(
    session: &mut GrapheneSession,
    account_name_or_id: String,
    limit: u32,
    offset: u32,
) -> Result<AccountHistorySubscription<'_>, SwaplockApiError> {
    validate_account_history_window(limit, offset)?;

    session.database_call(
        "set_subscribe_callback",
        json!([ACCOUNT_HISTORY_CALLBACK_ID, false]),
    )?;
    session.database_call("get_full_accounts", json!([[account_name_or_id], true]))?;

    let initial = get_account_history_snapshot(session, &account_name_or_id, limit, offset).await?;

    Ok(AccountHistorySubscription {
        session,
        account_name_or_id,
        last_seen_operation_id: initial.newest_operation_id,
        initial: initial.page,
    })
}

async fn get_account_history_snapshot(
    session: &mut GrapheneSession,
    account_name_or_id: &str,
    limit: u32,
    offset: u32,
) -> Result<AccountHistorySnapshot, SwaplockApiError> {
    validate_account_history_window(limit, offset)?;

    let params = account_history_params(account_name_or_id, limit, offset)?;
    let value = session.history_call(ACCOUNT_HISTORY_METHOD, params)?;
    let raw_items = account_history_items_from_value(value)?;
    let newest_operation_id = raw_items.first().map(|item| item.id.0.clone());
    let page = account_history_page_from_items(raw_items, limit, offset);

    Ok(AccountHistorySnapshot {
        page,
        newest_operation_id,
    })
}

async fn get_recent_account_history_since(
    session: &mut GrapheneSession,
    account_name_or_id: &str,
    last_seen_operation_id: Option<&str>,
) -> Result<Vec<OperationHistoryObject>, SwaplockApiError> {
    let stop = last_seen_operation_id.unwrap_or(HISTORY_STOP_SENTINEL);
    let value = session.history_call(
        ACCOUNT_HISTORY_METHOD,
        json!([
            account_name_or_id,
            stop,
            MAX_ACCOUNT_HISTORY_LIMIT,
            HISTORY_START_SENTINEL
        ]),
    )?;
    account_history_items_from_value(value)
}

fn validate_account_history_window(limit: u32, offset: u32) -> Result<(), SwaplockApiError> {
    let Some(window) = offset.checked_add(limit) else {
        return Err(SwaplockApiError::InvalidLimit {
            method: ACCOUNT_HISTORY_METHOD,
            limit,
            max: MAX_ACCOUNT_HISTORY_LIMIT.saturating_sub(offset),
        });
    };

    if limit == 0 || window > MAX_ACCOUNT_HISTORY_LIMIT {
        return Err(SwaplockApiError::InvalidLimit {
            method: ACCOUNT_HISTORY_METHOD,
            limit,
            max: MAX_ACCOUNT_HISTORY_LIMIT.saturating_sub(offset),
        });
    }
    Ok(())
}

fn account_history_params(
    account_name_or_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Value, SwaplockApiError> {
    validate_account_history_window(limit, offset)?;

    let wire_limit = offset + limit + 1;

    Ok(json!([
        account_name_or_id,
        HISTORY_STOP_SENTINEL,
        wire_limit,
        HISTORY_START_SENTINEL
    ]))
}

fn account_history_items_from_value(
    value: Value,
) -> Result<Vec<OperationHistoryObject>, SwaplockApiError> {
    serde_json::from_value(value).map_err(|error| SwaplockApiError::UnexpectedResponse {
        method: ACCOUNT_HISTORY_METHOD,
        message: error.to_string(),
    })
}

fn account_history_page_from_items(
    raw_items: Vec<OperationHistoryObject>,
    limit: u32,
    offset: u32,
) -> AccountHistoryPage {
    let total_needed = offset.saturating_add(limit) as usize;
    let has_more = raw_items.len() > total_needed;
    let items = raw_items
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect::<Vec<_>>();
    let next_offset = has_more.then_some(offset + limit);

    AccountHistoryPage {
        items,
        limit,
        offset,
        next_offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn parsed_history(ids: &[&str]) -> Vec<OperationHistoryObject> {
        account_history_items_from_value(Value::Array(
            ids.iter().map(|id| operation_history_fixture(id)).collect(),
        ))
        .unwrap()
    }

    #[test]
    fn first_account_history_page_uses_offset_lookahead_wire_shape() {
        assert_eq!(
            account_history_params("swaplock", 20, 0).unwrap(),
            json!(["swaplock", "1.11.0", 21, "1.11.0"])
        );
    }

    #[test]
    fn offset_account_history_page_overscans_to_requested_window() {
        assert_eq!(
            account_history_params("1.2.100", 5, 10).unwrap(),
            json!(["1.2.100", "1.11.0", 16, "1.11.0"])
        );
    }

    #[test]
    fn rejects_zero_limit_and_too_large_history_windows() {
        assert!(matches!(
            account_history_params("1.2.100", 0, 0),
            Err(SwaplockApiError::InvalidLimit { limit: 0, .. })
        ));
        assert!(matches!(
            account_history_params("1.2.100", MAX_ACCOUNT_HISTORY_LIMIT, 1),
            Err(SwaplockApiError::InvalidLimit {
                limit: MAX_ACCOUNT_HISTORY_LIMIT,
                max: 97,
                ..
            })
        ));
    }

    #[test]
    fn parses_generated_operation_history_objects() {
        let items = parsed_history(&["1.11.10"]);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id.0, "1.11.10");
        assert_eq!(items[0].block_num, 1);
        assert_eq!(items[0].block_time, "2026-01-01T00:00:00");
    }

    #[test]
    fn first_history_page_sets_next_offset_when_lookahead_exists() {
        let page =
            account_history_page_from_items(parsed_history(&["1.11.10", "1.11.9", "1.11.8"]), 2, 0);

        assert_eq!(page.limit(), 2);
        assert_eq!(page.offset(), 0);
        assert_eq!(page.items().len(), 2);
        assert_eq!(page.items()[0].id.0, "1.11.10");
        assert_eq!(page.items()[1].id.0, "1.11.9");
        assert!(page.has_more());
        assert_eq!(page.next_offset(), Some(2));
    }

    #[test]
    fn offset_history_page_skips_items_and_sets_next_offset() {
        let page = account_history_page_from_items(
            parsed_history(&["1.11.10", "1.11.9", "1.11.8", "1.11.7", "1.11.6"]),
            2,
            2,
        );

        assert_eq!(page.offset(), 2);
        assert_eq!(page.items().len(), 2);
        assert_eq!(page.items()[0].id.0, "1.11.8");
        assert_eq!(page.items()[1].id.0, "1.11.7");
        assert_eq!(page.next_offset(), Some(4));
    }

    #[test]
    fn history_page_has_no_next_offset_when_shorter_than_limit() {
        let page = account_history_page_from_items(parsed_history(&["1.11.10"]), 2, 0);

        assert_eq!(page.items().len(), 1);
        assert_eq!(page.next_offset(), None);
        assert!(!page.has_more());
    }

    #[test]
    fn history_page_empty_when_offset_exceeds_loaded_items() {
        let page = account_history_page_from_items(parsed_history(&["1.11.10"]), 2, 5);

        assert!(page.items().is_empty());
        assert_eq!(page.next_offset(), None);
    }
}
