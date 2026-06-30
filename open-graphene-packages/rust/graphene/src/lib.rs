mod chain;
mod client;
mod error;

pub use chain::{Acta, ActaClient, BitShares, BitSharesClient, ChainClientBuilder, Swaplock};
pub use client::{GrapheneClient, GrapheneClientBuilder, GrapheneClientConfig};
pub use error::{GrapheneConfigError, GrapheneConnectError};
pub use graphene_chain_swaplock_api::{
    AccountCreateRequest, AccountHistoryPage, AccountHistorySubscription, AccountId,
    AccountTransferRequest, AccountUpdateRequest, AccountUpgradeRequest, AccountWhitelistRequest,
    ArgumentType, AssertRequest, Asset, AssetClaimFeesRequest, AssetClaimPoolRequest,
    AssetCreateRequest, AssetFundFeePoolRequest, AssetGlobalSettleRequest, AssetId,
    AssetIssueOperation, AssetIssueRequest, AssetObject, AssetPublishFeedRequest,
    AssetReserveOperation, AssetReserveRequest, AssetSettleRequest, AssetUpdateBitassetRequest,
    AssetUpdateFeedProducersRequest, AssetUpdateIssuerRequest, AssetUpdateRequest, Authority,
    BalanceClaimRequest, BidCollateralRequest, BlindRequest, BlindSumRequest, BlindTransferRequest,
    BlindingFactor, BroadcastConfirmation, BroadcastReceipt, CallOrderUpdateRequest,
    ChainParameters, ChainPropertyObject, ChainStore, Commitment, CommitteeMemberCreateRequest,
    CommitteeMemberUpdateGlobalParametersRequest, CommitteeMemberUpdateRequest, ConnectionStrategy,
    CreditDealRepayRequest, CreditDealUpdateRequest, CreditOfferAcceptRequest,
    CreditOfferCreateRequest, CreditOfferDeleteRequest, CreditOfferUpdateRequest, CryptoApi,
    CustomAuthorityCreateRequest, CustomAuthorityDeleteRequest, CustomAuthorityUpdateRequest,
    CustomRequest, DEFAULT_ACCOUNT_HISTORY_LIMIT, DEFAULT_ACCOUNT_HISTORY_OFFSET,
    DEFAULT_FILL_ORDER_HISTORY_LIMIT, DEFAULT_GET_LIMIT_ORDERS_LIMIT,
    DEFAULT_GROUPED_LIMIT_ORDERS_LIMIT, DEFAULT_LIST_ASSETS_LIMIT, DEFAULT_LOOKUP_ACCOUNTS_LIMIT,
    DEFAULT_MARKET_HISTORY_BUCKET_SECONDS, DynamicGlobalPropertyObject, FillOrderHistoryRequest,
    GetBlockHeaderRequest, GetBlockRequest, GetConfigRequest, GetKeyReferencesRequest,
    GetLimitOrdersRequest, GetObjectsRequest, GetTickerRequest, GlobalPropertyObject,
    GroupedLimitOrdersRequest, HistoryKey, HtlcCreateRequest, HtlcExtendRequest, HtlcRedeemRequest,
    LimitOrderCancelOperation, LimitOrderCancelRequest, LimitOrderCreateOperation,
    LimitOrderCreateRequest, LimitOrderGroup, LimitOrderId, LimitOrderObject,
    LimitOrderUpdateOperation, LimitOrderUpdateRequest, LiquidityPoolCreateRequest,
    LiquidityPoolDeleteRequest, LiquidityPoolDepositRequest, LiquidityPoolExchangeRequest,
    LiquidityPoolUpdateRequest, LiquidityPoolWithdrawRequest, ListAssetsRequest,
    LookupAccountsRequest, MAX_ACCOUNT_HISTORY_LIMIT, MAX_FILL_ORDER_HISTORY_LIMIT,
    MarketHistoryRequest, MaybeSignedBlockHeader, NetworkBroadcastApi, Operation,
    OperationHistoryObject, OrderHistoryObject, OrdersApi, OverrideTransferRequest,
    PreparedTransaction, PreparedTransfer, Price, ProcessedTransaction, ProposalCreateRequest,
    ProposalDeleteRequest, ProposalUpdateRequest, RangeGetInfoRequest, RangeProof, RangeProofInfo,
    RangeProofSignRequest, ReconnectPolicy, Restriction, SWAPLOCK_CHAIN_ID, SametFundBorrowRequest,
    SametFundCreateRequest, SametFundDeleteRequest, SametFundRepayRequest, SametFundUpdateRequest,
    ServerLatency, SignedBlock, SignedTransactionEnvelope, SignedTransfer, SwaplockApi,
    SwaplockApiError, SwaplockLiveAccountBalancesByIdRequest,
    SwaplockLiveAccountBalancesSubscription, SwaplockLiveAccountByIdRequest,
    SwaplockLiveAccountHistoryByIdRequest, SwaplockLiveAccountHistoryRequest,
    SwaplockLiveAccountHistorySubscription, SwaplockLiveAccountOrdersByIdRequest,
    SwaplockLiveAccountOrdersSubscription, SwaplockLiveAccountSubscription, SwaplockLiveApi,
    SwaplockLiveAssetByIdRequest, SwaplockLiveAssetSubscription, SwaplockLiveDatabaseApi,
    SwaplockLiveDynamicGlobalPropertiesSubscription, SwaplockLiveHistoryApi,
    SwaplockLiveMarketSubscription, SwaplockLiveNetworkBroadcastApi,
    SwaplockLivePendingBroadcastConfirmation, Ticker, TicketCreateRequest, TicketUpdateRequest,
    TrackedGroupsRequest, TransactionBuilder, TransferFromBlindRequest, TransferOperation,
    TransferRequest, TransferToBlindRequest, VerifyRangeProofRewindRequest,
    VerifyRangeProofRewindResult, VerifyRangeRequest, VerifyRangeResult, VerifySumRequest,
    VestingBalanceCreateRequest, VestingBalanceWithdrawRequest, WithdrawPermissionClaimRequest,
    WithdrawPermissionCreateRequest, WithdrawPermissionDeleteRequest,
    WithdrawPermissionUpdateRequest, WitnessCreateRequest, WitnessUpdateRequest,
    WorkerCreateRequest, is_account_name, is_account_name_allow_short, is_cheap_name,
};
pub use open_graphene_fc::hash;
pub use open_graphene_fc::{
    AccountKeys, Address, BrainKey, PrivateKey, PublicKey, SUGGESTED_BRAIN_KEY_WORDS, Signature,
    account_role_key, decrypt_with_checksum, encrypt_with_checksum,
};

pub struct Graphene;

impl Graphene {
    pub fn builder() -> GrapheneClientBuilder {
        GrapheneClientBuilder::new()
    }

    pub fn swaplock() -> ChainClientBuilder<Swaplock> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }

    pub fn bitshares() -> ChainClientBuilder<BitShares> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }

    pub fn acta() -> ChainClientBuilder<Acta> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_builder_selects_swaplock_api() {
        let builder = Graphene::builder()
            .servers(["wss://node01.swaplock.chainpool.online:8090"])
            .chain_id("chain-id")
            .prefix("BTS")
            .build()
            .expect("valid generic Graphene config")
            .swaplock();

        assert_eq!(
            builder.config().servers(),
            &["wss://node01.swaplock.chainpool.online:8090".to_string()]
        );
        assert_eq!(builder.config().chain_id(), Some("chain-id"));
        assert_eq!(builder.config().prefix(), Some("BTS"));
    }

    #[test]
    fn chain_first_builder_collects_servers() {
        let builder = Graphene::swaplock()
            .server("wss://node01.swaplock.chainpool.online:8090")
            .server("wss://node02.swaplock.chainpool.online:8090")
            .chain_id("chain-id")
            .prefix("BTS")
            .build()
            .expect("valid Swaplock config");

        assert_eq!(builder.config().servers().len(), 2);
        assert_eq!(builder.config().chain_id(), Some("chain-id"));
        assert_eq!(builder.config().prefix(), Some("BTS"));
    }

    #[test]
    fn builder_rejects_missing_servers() {
        let error = Graphene::builder()
            .chain_id("chain-id")
            .prefix("BTS")
            .build()
            .expect_err("servers are required");

        assert_eq!(error, GrapheneConfigError::MissingServers);
    }
}
