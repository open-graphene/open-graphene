use graphene_chain_swaplock_bindings::generated::AccountObject;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_accounts as rpc_get_accounts;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

pub struct AccountsRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) names_or_ids: Vec<String>,
}

impl AccountsRequest<'_> {
    pub async fn get(self) -> Result<Vec<Option<AccountObject>>, SwaplockApiError> {
        let params = rpc_get_accounts::Params {
            account_names_or_ids: self.names_or_ids,
            subscribe: Some(false),
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(rpc_get_accounts::METHOD))?;
        let value = self
            .session
            .database_call(rpc_get_accounts::METHOD, params)
            .await?;
        rpc_get_accounts::parse_returns(value)
            .map_err(SwaplockApiError::unexpected(rpc_get_accounts::METHOD))
    }
}
