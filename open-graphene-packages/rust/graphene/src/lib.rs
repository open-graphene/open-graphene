mod chain;
mod client;
mod error;

pub use chain::{Acta, ActaClient, BitShares, BitSharesClient, ChainClientBuilder, Swaplock};
pub use client::{GrapheneClient, GrapheneClientBuilder, GrapheneClientConfig};
pub use error::{GrapheneConfigError, GrapheneConnectError};
pub use graphene_chain_swaplock_api::{
    AccountHistoryPage, AccountHistorySubscription, AccountId, Asset, AssetId, BlindRequest,
    BlindSumRequest, BlindingFactor, BroadcastConfirmation, BroadcastReceipt, Commitment,
    ConnectionStrategy, CryptoApi, DEFAULT_ACCOUNT_HISTORY_LIMIT, DEFAULT_ACCOUNT_HISTORY_OFFSET,
    DEFAULT_GROUPED_LIMIT_ORDERS_LIMIT, DEFAULT_LIST_ASSETS_LIMIT, DEFAULT_LOOKUP_ACCOUNTS_LIMIT,
    GroupedLimitOrdersRequest, LimitOrderCancelOperation, LimitOrderCancelRequest,
    LimitOrderCreateOperation, LimitOrderCreateRequest, LimitOrderGroup, LimitOrderId,
    ListAssetsRequest, LookupAccountsRequest, MAX_ACCOUNT_HISTORY_LIMIT, NetworkBroadcastApi,
    Operation, OrdersApi, PreparedTransaction, PreparedTransfer, RangeGetInfoRequest, RangeProof,
    RangeProofInfo, RangeProofSignRequest, SWAPLOCK_CHAIN_ID, ServerLatency,
    SignedTransactionEnvelope, SignedTransfer, SwaplockApi, SwaplockApiError,
    SwaplockLiveAccountBalancesByIdRequest, SwaplockLiveAccountBalancesSubscription,
    SwaplockLiveAccountByIdRequest, SwaplockLiveAccountHistoryByIdRequest,
    SwaplockLiveAccountHistoryRequest, SwaplockLiveAccountHistorySubscription,
    SwaplockLiveAccountOrdersByIdRequest, SwaplockLiveAccountOrdersSubscription,
    SwaplockLiveAccountSubscription, SwaplockLiveApi, SwaplockLiveAssetByIdRequest,
    SwaplockLiveAssetSubscription, SwaplockLiveDatabaseApi,
    SwaplockLiveDynamicGlobalPropertiesSubscription, SwaplockLiveHistoryApi,
    SwaplockLiveNetworkBroadcastApi, SwaplockLivePendingBroadcastConfirmation,
    TrackedGroupsRequest, TransactionBuilder, TransferOperation, TransferRequest,
    VerifyRangeProofRewindRequest, VerifyRangeProofRewindResult, VerifyRangeRequest,
    VerifyRangeResult, VerifySumRequest,
};
pub use open_graphene_fc::{
    AccountKeys, BrainKey, PrivateKey, PublicKey, account_role_key, decrypt_with_checksum,
    encrypt_with_checksum,
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
