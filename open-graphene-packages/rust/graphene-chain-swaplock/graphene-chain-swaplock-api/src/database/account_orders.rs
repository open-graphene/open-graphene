use graphene_chain_swaplock_bindings::generated::LimitOrderObject;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::account_by_name::get_account_by_name;
use super::account_orders_by_id::{
    AccountOrdersSubscription, get_account_orders_by_id, subscribe_account_orders_by_id,
};

pub struct AccountOrdersRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_name: String,
}

impl<'session> AccountOrdersRequest<'session> {
    pub async fn get(self) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        get_account_orders_by_id(self.session, &account.id.0).await
    }

    pub async fn subscribe(self) -> Result<AccountOrdersSubscription<'session>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        subscribe_account_orders_by_id(self.session, account.id.0).await
    }
}
