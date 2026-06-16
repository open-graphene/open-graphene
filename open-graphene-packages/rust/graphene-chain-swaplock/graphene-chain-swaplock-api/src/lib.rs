mod codec;
mod crypto;
mod database;
mod history;
mod live;
mod network_broadcast;
mod operations;
mod orders;

use open_graphene_core::{AmountError, BalanceError, HeaderError, ObjectIdError};
use open_graphene_transport::{GrapheneSession, TransportError};
use thiserror::Error;

pub use open_graphene_transport::{
    ChainIdMismatch, ConnectionStrategy, ServerConnectFailure, ServerLatency,
};

pub use crypto::{
    BlindRequest, BlindSumRequest, BlindingFactor, Commitment, CryptoApi, RangeGetInfoRequest,
    RangeProof, RangeProofInfo, RangeProofSignRequest, VerifyRangeProofRewindRequest,
    VerifyRangeProofRewindResult, VerifyRangeRequest, VerifyRangeResult, VerifySumRequest,
};

pub use live::{
    SwaplockLiveAccountBalancesByIdRequest, SwaplockLiveAccountBalancesSubscription,
    SwaplockLiveAccountByIdRequest, SwaplockLiveAccountHistoryByIdRequest,
    SwaplockLiveAccountHistoryRequest, SwaplockLiveAccountHistorySubscription,
    SwaplockLiveAccountOrdersByIdRequest, SwaplockLiveAccountOrdersSubscription,
    SwaplockLiveAccountSubscription, SwaplockLiveApi, SwaplockLiveAssetByIdRequest,
    SwaplockLiveAssetSubscription, SwaplockLiveDatabaseApi,
    SwaplockLiveDynamicGlobalPropertiesSubscription, SwaplockLiveHistoryApi,
    SwaplockLiveNetworkBroadcastApi, SwaplockLivePendingBroadcastConfirmation,
};
pub use network_broadcast::{
    BroadcastConfirmation, BroadcastReceipt, NetworkBroadcastApi, PendingBroadcastConfirmation,
};
pub use operations::{
    LimitOrderCancelRequest, LimitOrderCreateRequest, LimitOrderUpdateRequest, OperationsApi,
    PreparedTransaction, PreparedTransfer, SignedTransactionEnvelope, SignedTransfer,
    TransactionBuilder, TransferRequest,
};

// Binding types callers need to construct operations for `OperationsApi::transaction()`.
pub use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, LimitOrderId};
pub use graphene_chain_swaplock_bindings::generated::operations::{
    LimitOrderCancelOperation, LimitOrderCreateOperation, LimitOrderUpdateOperation,
    TransferOperation,
};
pub use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
pub use graphene_chain_swaplock_bindings::generated::types::{Asset, Price};

pub use orders::{
    DEFAULT_GROUPED_LIMIT_ORDERS_LIMIT, GroupedLimitOrdersRequest, LimitOrderGroup, OrdersApi,
    TrackedGroupsRequest,
};

pub use history::{
    AccountHistoryByIdRequest, AccountHistoryPage, AccountHistoryRequest,
    AccountHistorySubscription, DEFAULT_ACCOUNT_HISTORY_LIMIT, DEFAULT_ACCOUNT_HISTORY_OFFSET,
    HistoryApi, MAX_ACCOUNT_HISTORY_LIMIT,
};

pub use database::{
    AccountBalancesByIdRequest, AccountBalancesRequest, AccountBalancesSubscription,
    AccountByIdRequest, AccountByNameRequest, AccountOrdersByIdRequest, AccountOrdersRequest,
    AccountOrdersSubscription, AccountSubscription, AccountsRequest, AssetByIdRequest,
    AssetBySymbolRequest, AssetSubscription, ChainIdRequest, ChainPropertiesRequest,
    DEFAULT_LIST_ASSETS_LIMIT, DEFAULT_LOOKUP_ACCOUNTS_LIMIT, DatabaseApi,
    DynamicGlobalPropertiesRequest, DynamicGlobalPropertiesSubscription, GlobalPropertiesRequest,
    IntoStringList, ListAssetsRequest, LookupAccountsRequest,
};

pub const SWAPLOCK_CHAIN_ID: &str =
    "2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098";

#[derive(Debug, Error)]
pub enum SwaplockApiError {
    #[error("account `{account}` was not found")]
    AccountNotFound { account: String },

    #[error("asset `{asset}` was not found")]
    AssetNotFound { asset: String },

    #[error("invalid `{method}` limit {limit}; expected 1..={max}")]
    InvalidLimit {
        method: &'static str,
        limit: u32,
        max: u32,
    },

    #[error("unexpected `{method}` response: {message}")]
    UnexpectedResponse {
        method: &'static str,
        message: String,
    },

    #[error("invalid hex for `{kind}`: {message}")]
    InvalidHex { kind: &'static str, message: String },

    #[error("missing transfer field `{field}`")]
    MissingTransferField { field: &'static str },

    #[error("invalid transfer: {message}")]
    InvalidTransfer { message: String },

    #[error("required transfer fee {required} exceeds max fee {max}")]
    TransferFeeTooHigh { required: i64, max: i64 },

    #[error(transparent)]
    Header(#[from] HeaderError),

    #[error(transparent)]
    Amount(#[from] AmountError),

    #[error(transparent)]
    Balance(#[from] BalanceError),

    #[error(transparent)]
    ObjectId(#[from] ObjectIdError),

    #[error(transparent)]
    Transport(#[from] TransportError),
}

pub struct SwaplockApi {
    session: GrapheneSession,
}

impl SwaplockApi {
    /// Open a Swaplock session, trying `servers` in order and keeping the first that answers.
    ///
    /// Pass `expected_chain_id` to refuse a node on the wrong chain. For latency-based
    /// selection across many nodes, use [`connect_with_strategy`](Self::connect_with_strategy).
    pub async fn connect<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
    ) -> Result<Self, SwaplockApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::connect_with_strategy(
            servers,
            expected_chain_id,
            ConnectionStrategy::FirstAvailable,
        )
        .await
    }

    /// Open a Swaplock session, choosing the node according to `strategy`
    /// (first to answer, or lowest connect latency).
    ///
    /// Pass `expected_chain_id` to refuse a node on the wrong chain.
    pub async fn connect_with_strategy<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
        strategy: ConnectionStrategy,
    ) -> Result<Self, SwaplockApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let session = GrapheneSession::connect_with_strategy(servers, expected_chain_id, strategy)?;
        Ok(Self { session })
    }

    /// Rank `servers` by how fast each completes a connection, fastest first.
    ///
    /// A health check that opens and closes a session per node without holding a connection.
    /// Use it to drive a node picker or status panel, then call
    /// [`connect_with_strategy`](Self::connect_with_strategy) to connect.
    pub async fn probe_latencies<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
    ) -> Result<Vec<ServerLatency>, SwaplockApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Ok(GrapheneSession::probe_latencies(
            servers,
            expected_chain_id,
        )?)
    }

    pub fn database(&mut self) -> DatabaseApi<'_> {
        DatabaseApi {
            session: &mut self.session,
        }
    }

    pub fn history(&mut self) -> HistoryApi<'_> {
        HistoryApi {
            session: &mut self.session,
        }
    }

    pub fn operations(&mut self) -> OperationsApi<'_> {
        OperationsApi {
            session: &mut self.session,
        }
    }

    pub fn network_broadcast(&mut self) -> NetworkBroadcastApi<'_> {
        NetworkBroadcastApi {
            session: &mut self.session,
        }
    }

    pub fn crypto(&mut self) -> CryptoApi<'_> {
        CryptoApi {
            session: &mut self.session,
        }
    }

    pub fn orders(&mut self) -> OrdersApi<'_> {
        OrdersApi {
            session: &mut self.session,
        }
    }

    pub fn into_live(self) -> Result<SwaplockLiveApi, SwaplockApiError> {
        SwaplockLiveApi::from_session(self.session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_not_found_error_names_asset() {
        let error = SwaplockApiError::AssetNotFound {
            asset: "BTS".to_string(),
        };

        assert_eq!(error.to_string(), "asset `BTS` was not found");
    }
}
