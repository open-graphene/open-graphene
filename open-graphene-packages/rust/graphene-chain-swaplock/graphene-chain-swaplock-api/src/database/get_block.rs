use graphene_chain_swaplock_bindings::generated::rpc::database::get_block as rpc_get_block;
use graphene_chain_swaplock_bindings::generated::types::SignedBlock;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_block`: fetch a produced block by its height.
///
/// You get back `Some(block)` once the height has been produced, or `None` for a height the chain
/// has not reached yet. The block is the typed [`SignedBlock`] (witness, timestamp, transactions,
/// witness signature, merkle root).
pub struct GetBlockRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) block_num: u32,
}

impl GetBlockRequest<'_> {
    pub async fn get(self) -> Result<Option<SignedBlock>, SwaplockApiError> {
        let params = rpc_get_block::Params {
            block_num: self.block_num,
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(rpc_get_block::METHOD))?;
        let value = self
            .session
            .database_call(rpc_get_block::METHOD, params)
            .await?;
        rpc_get_block::parse_returns(value)
            .map_err(SwaplockApiError::unexpected(rpc_get_block::METHOD))
    }
}
