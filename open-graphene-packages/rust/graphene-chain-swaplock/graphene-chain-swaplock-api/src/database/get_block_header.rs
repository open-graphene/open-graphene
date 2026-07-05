use graphene_chain_swaplock_bindings::generated::rpc::database::get_block_header as rpc_get_block_header;
use graphene_chain_swaplock_bindings::generated::types::MaybeSignedBlockHeader;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

/// Builder for `get_block_header`: the header of a produced block, without its transactions.
///
/// Lighter than [`block`](crate::DatabaseApi::block) when you only need the metadata (witness,
/// timestamp, previous id). `Some(header)` once the height is produced, `None` past the head. The
/// header is the typed [`MaybeSignedBlockHeader`] (the witness signature is optional).
pub struct GetBlockHeaderRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) block_num: u32,
}

impl GetBlockHeaderRequest<'_> {
    pub async fn get(self) -> Result<Option<MaybeSignedBlockHeader>, SwaplockApiError> {
        let params = rpc_get_block_header::Params {
            block_num: self.block_num,
            with_witness_signature: None,
        }
        .to_params_value()
        .map_err(SwaplockApiError::unexpected(rpc_get_block_header::METHOD))?;
        let value = self
            .session
            .database_call(rpc_get_block_header::METHOD, params)
            .await?;
        rpc_get_block_header::parse_returns(value)
            .map_err(SwaplockApiError::unexpected(rpc_get_block_header::METHOD))
    }
}
