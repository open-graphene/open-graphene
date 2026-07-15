use graphene_chain_swaplock_bindings::generated::types::ProposalObject;
use open_graphene_transport::GrapheneSession;
use serde_json::json;

use crate::SwaplockApiError;

const METHOD: &str = "get_proposed_transactions";

/// Builder for `get_proposed_transactions`: live proposals relevant to an account —
/// those it proposed plus those awaiting its (active or owner) approval.
pub struct ProposedTransactionsRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) account_name_or_id: String,
}

impl ProposedTransactionsRequest<'_> {
    pub async fn get(self) -> Result<Vec<ProposalObject>, SwaplockApiError> {
        let value = self
            .session
            .database_call(METHOD, json!([self.account_name_or_id]))
            .await?;
        serde_json::from_value(value).map_err(SwaplockApiError::unexpected(METHOD))
    }
}
