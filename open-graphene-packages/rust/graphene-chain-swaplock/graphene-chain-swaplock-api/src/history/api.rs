use open_graphene_transport::GrapheneSession;

use super::account_history::AccountHistoryRequest;
use super::account_history_by_id::AccountHistoryByIdRequest;
use super::constants::{DEFAULT_ACCOUNT_HISTORY_LIMIT, DEFAULT_ACCOUNT_HISTORY_OFFSET};

pub struct HistoryApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
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
