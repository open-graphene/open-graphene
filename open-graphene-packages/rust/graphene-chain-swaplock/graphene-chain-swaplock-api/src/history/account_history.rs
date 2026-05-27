use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::page::AccountHistoryPage;
use super::subscription::{AccountHistorySubscription, subscribe_account_history};
use super::wire::get_account_history_snapshot;

pub struct AccountHistoryRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_name_or_id: String,
    pub(super) limit: u32,
    pub(super) offset: u32,
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
