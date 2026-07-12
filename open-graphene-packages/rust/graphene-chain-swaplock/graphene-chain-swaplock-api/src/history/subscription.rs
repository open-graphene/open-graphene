use graphene_chain_swaplock_bindings::generated::OperationHistoryObject;
use open_graphene_transport::{CallbackId, GrapheneSession, JsonRpcInbound};
use serde_json::json;

use crate::SwaplockApiError;

use super::constants::ACCOUNT_HISTORY_CALLBACK_ID;
use super::page::AccountHistoryPage;
use super::wire::{get_account_history_snapshot, get_recent_account_history_since};

pub struct AccountHistorySubscription<'session> {
    session: &'session mut GrapheneSession,
    account_name_or_id: String,
    initial: AccountHistoryPage,
    last_seen_operation_id: Option<String>,
}

impl AccountHistorySubscription<'_> {
    pub fn initial(&self) -> &AccountHistoryPage {
        &self.initial
    }

    pub async fn next_update(&mut self) -> Result<Vec<OperationHistoryObject>, SwaplockApiError> {
        loop {
            let notice = self.session.next_notice().await?;
            let JsonRpcInbound::Notice { callback_id, .. } = notice else {
                continue;
            };
            if callback_id != CallbackId::new(ACCOUNT_HISTORY_CALLBACK_ID) {
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

pub(super) async fn subscribe_account_history(
    session: &mut GrapheneSession,
    account_name_or_id: String,
    limit: u32,
    offset: u32,
) -> Result<AccountHistorySubscription<'_>, SwaplockApiError> {
    session
        .database_call(
            "set_subscribe_callback",
            json!([ACCOUNT_HISTORY_CALLBACK_ID, false]),
        )
        .await?;
    session
        .database_call("get_full_accounts", json!([[account_name_or_id], true]))
        .await?;

    let initial = get_account_history_snapshot(session, &account_name_or_id, limit, offset).await?;

    Ok(AccountHistorySubscription {
        session,
        account_name_or_id,
        last_seen_operation_id: initial.newest_operation_id,
        initial: initial.page,
    })
}
