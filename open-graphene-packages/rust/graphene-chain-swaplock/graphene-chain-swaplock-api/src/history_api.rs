use graphene_chain_swaplock_bindings::generated::OperationHistoryObject;
use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

pub const DEFAULT_ACCOUNT_HISTORY_LIMIT: u32 = 20;
pub const MAX_ACCOUNT_HISTORY_LIMIT: u32 = 98;

const ACCOUNT_HISTORY_METHOD: &str = "get_account_history";
const HISTORY_START_SENTINEL: &str = "1.11.0";
const HISTORY_STOP_SENTINEL: &str = "1.11.0";

pub struct HistoryApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

pub struct AccountHistoryRequest<'session> {
    session: &'session mut GrapheneSession,
    account_name_or_id: String,
    limit: u32,
    cursor: Option<AccountHistoryCursor>,
}

pub struct AccountHistoryByIdRequest<'session> {
    session: &'session mut GrapheneSession,
    account_id: String,
    limit: u32,
    cursor: Option<AccountHistoryCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountHistoryCursor {
    start_after: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccountHistoryPage {
    items: Vec<OperationHistoryObject>,
    next_cursor: Option<AccountHistoryCursor>,
}

impl AccountHistoryCursor {
    fn from_operation_id(id: impl Into<String>) -> Self {
        Self {
            start_after: id.into(),
        }
    }
}

impl AccountHistoryPage {
    pub fn items(&self) -> &[OperationHistoryObject] {
        &self.items
    }

    pub fn next_cursor(&self) -> Option<&AccountHistoryCursor> {
        self.next_cursor.as_ref()
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
            cursor: None,
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
            cursor: None,
        }
    }
}

impl<'session> AccountHistoryRequest<'session> {
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn cursor(mut self, cursor: AccountHistoryCursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn after(self, cursor: AccountHistoryCursor) -> Self {
        self.cursor(cursor)
    }

    pub async fn get(self) -> Result<AccountHistoryPage, SwaplockApiError> {
        get_account_history_page(
            self.session,
            self.account_name_or_id,
            self.limit,
            self.cursor,
        )
        .await
    }
}

impl<'session> AccountHistoryByIdRequest<'session> {
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn cursor(mut self, cursor: AccountHistoryCursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn after(self, cursor: AccountHistoryCursor) -> Self {
        self.cursor(cursor)
    }

    pub async fn get(self) -> Result<AccountHistoryPage, SwaplockApiError> {
        get_account_history_page(self.session, self.account_id, self.limit, self.cursor).await
    }
}

async fn get_account_history_page(
    session: &mut GrapheneSession,
    account_name_or_id: String,
    limit: u32,
    cursor: Option<AccountHistoryCursor>,
) -> Result<AccountHistoryPage, SwaplockApiError> {
    validate_account_history_limit(limit)?;

    let params = account_history_params(&account_name_or_id, limit, cursor.as_ref())?;
    let value = session.history_call(ACCOUNT_HISTORY_METHOD, params)?;
    let raw_items = account_history_items_from_value(value)?;

    Ok(account_history_page_from_items(
        raw_items,
        limit,
        cursor.as_ref(),
    ))
}

fn validate_account_history_limit(limit: u32) -> Result<(), SwaplockApiError> {
    if !(1..=MAX_ACCOUNT_HISTORY_LIMIT).contains(&limit) {
        return Err(SwaplockApiError::InvalidLimit {
            method: ACCOUNT_HISTORY_METHOD,
            limit,
            max: MAX_ACCOUNT_HISTORY_LIMIT,
        });
    }
    Ok(())
}

fn account_history_params(
    account_name_or_id: &str,
    limit: u32,
    cursor: Option<&AccountHistoryCursor>,
) -> Result<Value, SwaplockApiError> {
    validate_account_history_limit(limit)?;

    let wire_limit = match cursor {
        Some(_) => limit + 2,
        None => limit + 1,
    };
    let start = cursor
        .map(|cursor| cursor.start_after.as_str())
        .unwrap_or(HISTORY_START_SENTINEL);

    Ok(json!([
        account_name_or_id,
        HISTORY_STOP_SENTINEL,
        wire_limit,
        start
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
    cursor: Option<&AccountHistoryCursor>,
) -> AccountHistoryPage {
    let mut items = raw_items;
    if let Some(cursor) = cursor {
        if items
            .first()
            .is_some_and(|item| item.id.0 == cursor.start_after)
        {
            items.remove(0);
        }
    }

    let has_more = items.len() > limit as usize;
    items.truncate(limit as usize);
    let next_cursor = has_more
        .then(|| {
            items
                .last()
                .map(|item| AccountHistoryCursor::from_operation_id(&item.id.0))
        })
        .flatten();

    AccountHistoryPage { items, next_cursor }
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
    fn first_account_history_page_uses_lookahead_wire_shape() {
        assert_eq!(
            account_history_params("swaplock", 20, None).unwrap(),
            json!(["swaplock", "1.11.0", 21, "1.11.0"])
        );
    }

    #[test]
    fn cursor_account_history_page_requests_cursor_duplicate_and_lookahead() {
        let cursor = AccountHistoryCursor::from_operation_id("1.11.99");

        assert_eq!(
            account_history_params("1.2.100", 5, Some(&cursor)).unwrap(),
            json!(["1.2.100", "1.11.0", 7, "1.11.99"])
        );
    }

    #[test]
    fn rejects_zero_and_too_large_history_limits() {
        assert!(matches!(
            account_history_params("1.2.100", 0, None),
            Err(SwaplockApiError::InvalidLimit {
                limit: 0,
                max: MAX_ACCOUNT_HISTORY_LIMIT,
                ..
            })
        ));
        assert!(matches!(
            account_history_params("1.2.100", MAX_ACCOUNT_HISTORY_LIMIT + 1, None),
            Err(SwaplockApiError::InvalidLimit { limit, max: MAX_ACCOUNT_HISTORY_LIMIT, .. })
                if limit == MAX_ACCOUNT_HISTORY_LIMIT + 1
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
    fn first_history_page_sets_cursor_when_lookahead_exists() {
        let page = account_history_page_from_items(
            parsed_history(&["1.11.10", "1.11.9", "1.11.8"]),
            2,
            None,
        );

        assert_eq!(page.items().len(), 2);
        assert_eq!(page.items()[0].id.0, "1.11.10");
        assert_eq!(page.items()[1].id.0, "1.11.9");
        assert_eq!(
            page.next_cursor(),
            Some(&AccountHistoryCursor::from_operation_id("1.11.9"))
        );
    }

    #[test]
    fn history_page_has_no_cursor_when_shorter_than_limit() {
        let page = account_history_page_from_items(parsed_history(&["1.11.10"]), 2, None);

        assert_eq!(page.items().len(), 1);
        assert_eq!(page.next_cursor(), None);
    }

    #[test]
    fn cursor_history_page_drops_inclusive_cursor_duplicate() {
        let cursor = AccountHistoryCursor::from_operation_id("1.11.9");
        let page = account_history_page_from_items(
            parsed_history(&["1.11.9", "1.11.8", "1.11.7", "1.11.6"]),
            2,
            Some(&cursor),
        );

        assert_eq!(page.items().len(), 2);
        assert_eq!(page.items()[0].id.0, "1.11.8");
        assert_eq!(page.items()[1].id.0, "1.11.7");
        assert_eq!(
            page.next_cursor(),
            Some(&AccountHistoryCursor::from_operation_id("1.11.7"))
        );
    }
}
