use graphene_chain_swaplock_bindings::generated::Asset;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::account_balances_by_id::{
    AccountBalancesSubscription, get_account_balances_by_id, subscribe_account_balances_by_id,
};
use super::account_by_name::get_account_by_name;
use super::asset_by_symbol::get_asset_by_symbol;

pub struct AccountBalancesRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_name: String,
    pub(super) asset_symbols: Vec<String>,
}

impl<'session> AccountBalancesRequest<'session> {
    pub async fn get(self) -> Result<Vec<Asset>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        let mut asset_ids = Vec::new();
        for symbol in self.asset_symbols {
            let asset = get_asset_by_symbol(self.session, &symbol).await?;
            asset_ids.push(asset.id.0);
        }
        get_account_balances_by_id(self.session, &account.id.0, asset_ids).await
    }

    pub async fn subscribe(
        self,
    ) -> Result<AccountBalancesSubscription<'session>, SwaplockApiError> {
        let account = get_account_by_name(self.session, &self.account_name).await?;
        let mut asset_ids = Vec::new();
        for symbol in self.asset_symbols {
            let asset = get_asset_by_symbol(self.session, &symbol).await?;
            asset_ids.push(asset.id.0);
        }
        subscribe_account_balances_by_id(self.session, account.id.0, asset_ids).await
    }
}
