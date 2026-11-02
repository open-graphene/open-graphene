mod codec;
mod crypto;
mod database;
mod history;
mod live;
mod network_broadcast;
mod operations;
mod orders;

pub use bindings::generated::OperationHistoryObject;
use open_graphene_core::{AmountError, BalanceError, HeaderError, ObjectIdError};
use open_graphene_transport::{GrapheneSession, TransportError};
use thiserror::Error;

// Chain-agnostic account-name validation lives in core; re-exported here so it sits on the SDK
// surface next to the calls that take account names.
pub use open_graphene_core::{is_account_name, is_account_name_allow_short, is_cheap_name};

pub use open_graphene_transport::{
    ChainIdMismatch, ConnectionStrategy, ReconnectPolicy, ServerConnectFailure, ServerLatency,
};

pub use crypto::{
    BlindRequest, BlindSumRequest, BlindingFactor, Commitment, CryptoApi, RangeGetInfoRequest,
    RangeProof, RangeProofInfo, RangeProofSignRequest, VerifyRangeProofRewindRequest,
    VerifyRangeProofRewindResult, VerifyRangeRequest, VerifyRangeResult, VerifySumRequest,
};

pub use live::{
    ChainStore, ChainLiveAccountBalancesByIdRequest, ChainLiveAccountBalancesSubscription,
    ChainLiveAccountByIdRequest, ChainLiveAccountHistoryByIdRequest,
    ChainLiveAccountHistoryRequest, ChainLiveAccountHistorySubscription,
    ChainLiveAccountOrdersByIdRequest, ChainLiveAccountOrdersSubscription,
    ChainLiveAccountSubscription, ChainLiveApi, ChainLiveAssetByIdRequest,
    ChainLiveAssetSubscription, ChainLiveDatabaseApi,
    ChainLiveDynamicGlobalPropertiesSubscription, ChainLiveHistoryApi,
    ChainLiveMarketSubscription, ChainLiveNetworkBroadcastApi,
    ChainLivePendingBroadcastConfirmation,
};
pub use network_broadcast::{
    BroadcastConfirmation, BroadcastReceipt, NetworkBroadcastApi, PendingBroadcastConfirmation,
};
pub use operations::{
    AccountCreateRequest, AccountTransferRequest, AccountUpdateRequest, AccountUpgradeRequest,
    AccountWhitelistRequest, AssertRequest, AssetClaimFeesRequest, AssetClaimPoolRequest,
    AssetCreateRequest, AssetFundFeePoolRequest, AssetGlobalSettleRequest, AssetIssueRequest,
    AssetPublishFeedRequest, AssetReserveRequest, AssetSettleRequest, AssetUpdateBitassetRequest,
    AssetUpdateFeedProducersRequest, AssetUpdateIssuerRequest, AssetUpdateRequest,
    BalanceClaimRequest, CallOrderUpdateRequest,
    CommitteeMemberCreateRequest, CommitteeMemberUpdateGlobalParametersRequest,
    CommitteeMemberUpdateRequest, CustomAuthorityCreateRequest, CustomAuthorityDeleteRequest, CustomAuthorityUpdateRequest,
    CustomRequest, HtlcCreateRequest, HtlcExtendRequest, HtlcRedeemRequest,
    LimitOrderCancelRequest, LimitOrderCreateRequest, OperationsApi, OverrideTransferRequest, PreparedTransaction, PreparedTransfer,
    ProposalCreateRequest, ProposalDeleteRequest, ProposalUpdateRequest, SignedTransactionEnvelope, SignedTransfer, TicketCreateRequest, transaction_id,
    TicketUpdateRequest, TransactionBuilder, TransferRequest,
    VestingBalanceCreateRequest, VestingBalanceWithdrawRequest,
    WithdrawPermissionClaimRequest, WithdrawPermissionCreateRequest,
    WithdrawPermissionDeleteRequest, WithdrawPermissionUpdateRequest, WitnessCreateRequest,
    WitnessUpdateRequest, WorkerCreateRequest, };
#[cfg(feature = "bitshares7")]
pub use operations::{
    BidCollateralRequest, BlindTransferRequest, CreditDealRepayRequest, CreditDealUpdateRequest, CreditOfferAcceptRequest, CreditOfferCreateRequest, CreditOfferDeleteRequest, CreditOfferUpdateRequest, LimitOrderUpdateRequest, LiquidityPoolCreateRequest, LiquidityPoolDeleteRequest, LiquidityPoolDepositRequest, LiquidityPoolExchangeRequest, LiquidityPoolUpdateRequest, LiquidityPoolWithdrawRequest, SametFundBorrowRequest, SametFundCreateRequest, SametFundDeleteRequest, SametFundRepayRequest, SametFundUpdateRequest, TransferFromBlindRequest, TransferToBlindRequest,
};

// Binding types callers need to construct operations for `OperationsApi::transaction()`.
pub use bindings::generated::ids::{AccountId, AssetId, LimitOrderId};
pub use bindings::generated::operations::{
    AssetIssueOperation, AssetReserveOperation, LimitOrderCancelOperation,
    LimitOrderCreateOperation, TransferOperation,
};
pub use bindings::generated::static_variants::{
    ArgumentType, Operation,
};
pub use bindings::generated::types::{
    Asset, AssetObject, Authority, ChainParameters, ChainPropertyObject, DynamicGlobalPropertyObject,
    GlobalPropertyObject, LimitOrderObject, Price, ProcessedTransaction,
    Restriction, SignedBlock,
};

#[cfg(feature = "bitshares7")]
pub use bindings::generated::operations::LimitOrderUpdateOperation;
/// A block header as `get_block_header` returns it. BitShares 7 chains may include the witness
/// signature; older chains return the plain header under the same name.
#[cfg(feature = "bitshares7")]
pub use bindings::generated::types::MaybeSignedBlockHeader;
#[cfg(not(feature = "bitshares7"))]
pub use bindings::generated::types::BlockHeader as MaybeSignedBlockHeader;

pub use orders::{
    DEFAULT_GROUPED_LIMIT_ORDERS_LIMIT, GroupedLimitOrdersRequest, LimitOrderGroup, OrdersApi,
    TrackedGroupsRequest,
};

pub use history::{
    AccountHistoryByIdRequest, AccountHistoryPage, AccountHistoryRequest,
    AccountHistorySubscription, DEFAULT_ACCOUNT_HISTORY_LIMIT, DEFAULT_ACCOUNT_HISTORY_OFFSET,
    DEFAULT_FILL_ORDER_HISTORY_LIMIT, DEFAULT_MARKET_HISTORY_BUCKET_SECONDS,
    FillOrderHistoryRequest, HistoryApi, HistoryKey, MAX_ACCOUNT_HISTORY_LIMIT,
    MAX_FILL_ORDER_HISTORY_LIMIT, MarketHistoryRequest, OrderHistoryObject,
};

pub use database::{
    AccountBalancesByIdRequest, AccountBalancesRequest, AccountBalancesSubscription,
    AccountByIdRequest, AccountByNameRequest, AccountOrdersByIdRequest, AccountOrdersRequest,
    AccountOrdersSubscription, AccountSubscription, AccountsRequest, AssetByIdRequest,
    AssetBySymbolRequest, AssetSubscription, ChainIdRequest, ChainPropertiesRequest,
    DEFAULT_GET_LIMIT_ORDERS_LIMIT, DEFAULT_LIST_ASSETS_LIMIT, DEFAULT_LOOKUP_ACCOUNTS_LIMIT,
    DatabaseApi,
    DynamicGlobalPropertiesRequest, DynamicGlobalPropertiesSubscription, GetBlockHeaderRequest,
    GetBlockRequest, GetConfigRequest, GetKeyReferencesRequest, GetLimitOrdersRequest,
    GetObjectsRequest, GetTickerRequest, GlobalPropertiesRequest, IncludedTransaction, IntoStringList,
    ListAssetsRequest, LookupAccountsRequest, ProposedTransactionsRequest, Ticker,
};

/// The chain this crate is built for, from its generated bindings.
pub const CHAIN_NAME: &str = bindings::generated::ids::CHAIN_ID;
/// The chain id the bindings were generated against: SHA-256 of the genesis file.
pub const CHAIN_ID: &str = bindings::generated::ids::CHAIN_ID_HEX;
/// The prefix of public keys on this chain.
pub const PUBLIC_KEY_PREFIX: &str = bindings::generated::ids::PUBLIC_KEY_PREFIX;

/// Whether this chain's nodes accept a compact signature (`fc::ecc::public_key::is_canonical`):
/// low-S on chains built with the `low-s` feature (Actanet), the legacy Graphene rule elsewhere.
/// Signatures this crate makes pass both.
pub fn is_accepted_signature(signature: &[u8]) -> bool {
    #[cfg(feature = "low-s")]
    {
        open_graphene_fc::is_low_s_compact_signature(signature)
    }
    #[cfg(not(feature = "low-s"))]
    {
        bindings::generated::fc::is_graphene_canonical_compact_signature(signature)
    }
}

#[derive(Debug, Error)]
pub enum ChainApiError {
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

    #[error("required fees {required} exceed max fee {max}")]
    FeeTooHigh { required: i64, max: i64 },

    #[error("transaction {id} was not seen in a block before the timeout")]
    InclusionNotObserved { id: String },

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

impl ChainApiError {
    /// Standard mapping for serde failures around the generated RPC layer
    /// (`Params::to_params_value` / `parse_returns`).
    pub(crate) fn unexpected(method: &'static str) -> impl FnOnce(serde_json::Error) -> Self {
        move |error| Self::UnexpectedResponse {
            method,
            message: error.to_string(),
        }
    }
}

pub struct ChainApi {
    session: GrapheneSession,
}

impl ChainApi {
    /// Open a session, trying `servers` in order and keeping the first that answers.
    ///
    /// Pass `expected_chain_id` to refuse a node on the wrong chain. For latency-based
    /// selection across many nodes, use [`connect_with_strategy`](Self::connect_with_strategy).
    pub async fn connect<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
    ) -> Result<Self, ChainApiError>
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

    /// Open a session, choosing the node according to `strategy`
    /// (first to answer, or lowest connect latency).
    ///
    /// Pass `expected_chain_id` to refuse a node on the wrong chain.
    pub async fn connect_with_strategy<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
        strategy: ConnectionStrategy,
    ) -> Result<Self, ChainApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let session =
            GrapheneSession::connect_with_strategy(servers, expected_chain_id, strategy).await?;
        Ok(Self { session })
    }

    /// The chain this session is connected to — the genesis-derived id that
    /// distinguishes one network of this chain from another.
    ///
    /// Read from the session established at connect time, so it costs no RPC
    /// and cannot disagree with the node actually being talked to.
    pub fn chain_id(&self) -> &str {
        self.session.chain_id()
    }

    /// Rank `servers` by how fast each completes a connection, fastest first.
    ///
    /// A health check that opens and closes a session per node without holding a connection.
    /// Use it to drive a node picker or status panel, then call
    /// [`connect_with_strategy`](Self::connect_with_strategy) to connect.
    pub async fn probe_latencies<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
    ) -> Result<Vec<ServerLatency>, ChainApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Ok(GrapheneSession::probe_latencies(servers, expected_chain_id).await?)
    }

    /// Re-dial the same node and re-establish the API ids in place (refuses a different chain id).
    ///
    /// Read calls already reconnect themselves per the [`ReconnectPolicy`]; call this to force a
    /// reconnect, e.g. before resubscribing. Live subscriptions are not resumed automatically.
    pub async fn reconnect(&mut self) -> Result<(), ChainApiError> {
        self.session.reconnect().await?;
        Ok(())
    }

    /// Tune how read calls reconnect after a dropped connection
    /// (pass [`ReconnectPolicy::disabled`] to turn it off).
    pub fn set_reconnect_policy(&mut self, policy: ReconnectPolicy) {
        self.session.set_reconnect_policy(policy);
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

    pub fn into_live(self) -> Result<ChainLiveApi, ChainApiError> {
        ChainLiveApi::from_session(self.session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_not_found_error_names_asset() {
        let error = ChainApiError::AssetNotFound {
            asset: "BTS".to_string(),
        };

        assert_eq!(error.to_string(), "asset `BTS` was not found");
    }
}
