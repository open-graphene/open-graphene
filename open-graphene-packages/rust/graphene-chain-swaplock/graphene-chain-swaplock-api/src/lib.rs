mod codec;
mod crypto;
mod database;
mod history;
mod live;
mod member;
mod network_broadcast;
mod operations;
mod orders;

pub use graphene_chain_swaplock_bindings::generated::OperationHistoryObject;
use open_graphene_core::{AmountError, BalanceError, HeaderError, ObjectIdError};
use open_graphene_transport::{GrapheneSession, TransportError};
use thiserror::Error;

// Chain-agnostic account-name validation lives in core; re-exported here so it sits on the SDK
// surface next to the calls that take account names.
pub use open_graphene_core::{is_account_name, is_account_name_allow_short, is_cheap_name};

// A data room member may be an account or a bare public key; these read one from a string.
pub use member::{is_public_key, member_ref};

pub use open_graphene_transport::{
    ChainIdMismatch, ConnectionStrategy, ReconnectPolicy, ServerConnectFailure, ServerLatency,
};

pub use crypto::{
    BlindRequest, BlindSumRequest, BlindingFactor, Commitment, CryptoApi, RangeGetInfoRequest,
    RangeProof, RangeProofInfo, RangeProofSignRequest, VerifyRangeProofRewindRequest,
    VerifyRangeProofRewindResult, VerifyRangeRequest, VerifyRangeResult, VerifySumRequest,
};

pub use live::{
    ChainStore, SwaplockLiveAccountBalancesByIdRequest, SwaplockLiveAccountBalancesSubscription,
    SwaplockLiveAccountByIdRequest, SwaplockLiveAccountHistoryByIdRequest,
    SwaplockLiveAccountHistoryRequest, SwaplockLiveAccountHistorySubscription,
    SwaplockLiveAccountOrdersByIdRequest, SwaplockLiveAccountOrdersSubscription,
    SwaplockLiveAccountSubscription, SwaplockLiveApi, SwaplockLiveAssetByIdRequest,
    SwaplockLiveAssetSubscription, SwaplockLiveDatabaseApi,
    SwaplockLiveDynamicGlobalPropertiesSubscription, SwaplockLiveHistoryApi,
    SwaplockLiveMarketSubscription, SwaplockLiveNetworkBroadcastApi,
    SwaplockLivePendingBroadcastConfirmation,
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
    BalanceClaimRequest, BidCollateralRequest, BlindTransferRequest, CallOrderUpdateRequest,
    CommitteeMemberCreateRequest, CommitteeMemberUpdateGlobalParametersRequest,
    CommitteeMemberUpdateRequest, ContentCardCreateRequest, ContentCardGrantCreateRequest,
    ContentCardGrantRevokeRequest, ContentCardRemoveRequest, ContentCardUpdateRequest,
    CreditDealRepayRequest, CreditDealUpdateRequest, CreditOfferAcceptRequest,
    CreditOfferCreateRequest, CreditOfferDeleteRequest, CreditOfferUpdateRequest,
    CustomAuthorityCreateRequest, CustomAuthorityDeleteRequest, CustomAuthorityUpdateRequest,
    CustomRequest, DATA_ROOM_PERM_ADD_MEMBERS, DATA_ROOM_PERM_ALL, DATA_ROOM_PERM_CREATE_CONTENT,
    DATA_ROOM_PERM_GRANT_CONTENT, DATA_ROOM_PERM_MANAGE_CONTENT,
    DATA_ROOM_PERM_MANAGE_PERMISSIONS,
    DATA_ROOM_PERM_REMOVE_MEMBERS, DATA_ROOM_PERM_ROTATE_KEYS, DATA_ROOM_PERM_UPDATE_ROOM,
    DataRoomCreateRequest, DataRoomDeleteRequest, DataRoomMemberAddRequest,
    DataRoomMemberRemoveRequest, DataRoomMemberUpdateRequest, DataRoomRotateKeyRequest,
    DataRoomUpdateRequest, HtlcCreateRequest, HtlcExtendRequest, HtlcRedeemRequest,
    LimitOrderCancelRequest, LimitOrderCreateRequest, LimitOrderUpdateRequest,
    LiquidityPoolCreateRequest, LiquidityPoolDeleteRequest, LiquidityPoolDepositRequest,
    LiquidityPoolExchangeRequest, LiquidityPoolUpdateRequest, LiquidityPoolWithdrawRequest,
    OperationsApi, OverrideTransferRequest, PreparedTransaction, PreparedTransfer,
    ProposalCreateRequest, ProposalDeleteRequest, ProposalUpdateRequest, SametFundBorrowRequest,
    SametFundCreateRequest, SametFundDeleteRequest, SametFundRepayRequest, SametFundUpdateRequest,
    SignedTransactionEnvelope, SignedTransfer, TicketCreateRequest, TicketUpdateRequest,
    TransactionBuilder, TransferFromBlindRequest, TransferRequest, TransferToBlindRequest,
    VestingBalanceCreateRequest, VestingBalanceWithdrawRequest, WithdrawPermissionClaimRequest,
    WithdrawPermissionCreateRequest, WithdrawPermissionDeleteRequest,
    WithdrawPermissionUpdateRequest, WitnessCreateRequest, WitnessUpdateRequest,
    WorkerCreateRequest, content_card_grant_create_operation, member_add_operation,
    member_remove_operation, rotate_key_operation,
};

// Binding types callers need to construct operations for `OperationsApi::transaction()`.
pub use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, LimitOrderId};
pub use graphene_chain_swaplock_bindings::generated::operations::{
    AssetIssueOperation, AssetReserveOperation, LimitOrderCancelOperation,
    LimitOrderCreateOperation, LimitOrderUpdateOperation, TransferOperation,
};
pub use graphene_chain_swaplock_bindings::generated::static_variants::{
    ArgumentType, DataRoomSubject, Operation,
};
pub use graphene_chain_swaplock_bindings::generated::types::{
    Asset, AssetObject, Authority, ChainParameters, ChainPropertyObject, ContentCardObject,
    DataRoomKeyEpochObject, DataRoomMemberObject, DataRoomObject, DynamicGlobalPropertyObject,
    GlobalPropertyObject, LimitOrderObject, MaybeSignedBlockHeader, Price, ProcessedTransaction,
    Restriction, SignedBlock,
};

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
    ContentCardByIdRequest, ContentCardGrantRequest, ContentCardGrantsByCardRequest,
    ContentCardGrantsByGranteeRequest, ContentCardsByAuthorRequest, ContentCardsByRoomRequest,
    DEFAULT_GET_LIMIT_ORDERS_LIMIT, DEFAULT_LIST_ASSETS_LIMIT, DEFAULT_LOOKUP_ACCOUNTS_LIMIT,
    DataRoomByIdRequest, DataRoomKeyEpochRequest, DataRoomKeyEpochsRequest, DataRoomMemberRequest,
    DataRoomMembersRequest, DataRoomsByMemberRequest, DataRoomsByOwnerRequest,
    DataRoomsBySubjectRequest, DatabaseApi, DynamicGlobalPropertiesRequest,
    DynamicGlobalPropertiesSubscription, GetBlockHeaderRequest, GetBlockRequest, GetConfigRequest,
    GetKeyReferencesRequest, GetLimitOrdersRequest, GetObjectsRequest, GetTickerRequest,
    GlobalPropertiesRequest, IntoStringList, ListAssetsRequest, LookupAccountsRequest,
    ProposedTransactionsRequest, Ticker,
};

pub const SWAPLOCK_CHAIN_ID: &str =
    "e80d8f63b598759059ca8f8627a6c9252bf6ae13ed404e1afbf4ae51b1781837";

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

    #[error(
        "a key member cannot pay a fee, so `{operation}` needs an explicit payer account \
         alongside the public key"
    )]
    KeyMemberNeedsPayer { operation: &'static str },

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

impl SwaplockApiError {
    /// Standard mapping for serde failures around the generated RPC layer
    /// (`Params::to_params_value` / `parse_returns`).
    pub(crate) fn unexpected(method: &'static str) -> impl FnOnce(serde_json::Error) -> Self {
        move |error| Self::UnexpectedResponse {
            method,
            message: error.to_string(),
        }
    }
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
        let session =
            GrapheneSession::connect_with_strategy(servers, expected_chain_id, strategy).await?;
        Ok(Self { session })
    }

    /// The chain this session is connected to — the genesis-derived id that
    /// distinguishes one Swaplock network from another.
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
    ) -> Result<Vec<ServerLatency>, SwaplockApiError>
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
    pub async fn reconnect(&mut self) -> Result<(), SwaplockApiError> {
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
