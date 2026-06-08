mod connection;
mod database;
mod history;
mod live;
mod network_broadcast;
mod operations;

use std::time::Duration;

use open_graphene_core::{AmountError, BalanceError, HeaderError, ObjectIdError};
use open_graphene_transport::{GrapheneSession, TransportError};
use thiserror::Error;

pub use connection::{ConnectionStrategy, ServerLatency};

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
pub use operations::{OperationsApi, PreparedTransfer, SignedTransfer, TransferRequest};

pub use history::{
    AccountHistoryByIdRequest, AccountHistoryPage, AccountHistoryRequest,
    AccountHistorySubscription, DEFAULT_ACCOUNT_HISTORY_LIMIT, DEFAULT_ACCOUNT_HISTORY_OFFSET,
    HistoryApi, MAX_ACCOUNT_HISTORY_LIMIT,
};

pub use database::{
    AccountBalancesByIdRequest, AccountBalancesRequest, AccountBalancesSubscription,
    AccountByIdRequest, AccountByNameRequest, AccountOrdersByIdRequest, AccountOrdersRequest,
    AccountOrdersSubscription, AccountSubscription, AccountsRequest, AssetByIdRequest,
    AssetBySymbolRequest, AssetSubscription, ChainIdRequest, ChainPropertiesRequest, DatabaseApi,
    DynamicGlobalPropertiesRequest, DynamicGlobalPropertiesSubscription, GlobalPropertiesRequest,
    IntoStringList,
};

pub const SWAPLOCK_CHAIN_ID: &str =
    "2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConnectFailure {
    pub server: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainIdMismatch {
    pub server: String,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Error)]
pub enum SwaplockApiError {
    #[error("at least one Swaplock RPC server is required")]
    MissingServers,

    #[error("connected Swaplock RPC server returned unexpected chain id: {mismatch:?}")]
    ChainIdMismatch { mismatch: ChainIdMismatch },

    #[error("account `{account}` was not found")]
    AccountNotFound { account: String },

    #[error("asset `{asset}` was not found")]
    AssetNotFound { asset: String },

    #[error("all Swaplock RPC servers failed: {attempts:?}")]
    AllServersFailed { attempts: Vec<ServerConnectFailure> },

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
    /// Connect to the first server that answers, in the order given.
    ///
    /// Equivalent to [`connect_with_strategy`](Self::connect_with_strategy) with
    /// [`ConnectionStrategy::FirstAvailable`].
    pub async fn connect<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
    ) -> Result<Self, SwaplockApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::connect_with_strategy(servers, expected_chain_id, ConnectionStrategy::FirstAvailable)
            .await
    }

    /// Connect to one of `servers`, choosing which according to `strategy`.
    ///
    /// - [`ConnectionStrategy::FirstAvailable`] returns as soon as a server answers.
    /// - [`ConnectionStrategy::LowestLatency`] probes every server and keeps the fastest session.
    ///
    /// Unreachable servers are collected and, if none succeed, reported as
    /// [`SwaplockApiError::AllServersFailed`]. A chain-id mismatch aborts immediately.
    pub async fn connect_with_strategy<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
        strategy: ConnectionStrategy,
    ) -> Result<Self, SwaplockApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let servers = servers.into_iter().map(Into::into).collect::<Vec<_>>();
        if servers.is_empty() {
            return Err(SwaplockApiError::MissingServers);
        }

        let mut attempts = Vec::new();
        let mut best: Option<(Duration, GrapheneSession)> = None;
        for server in servers {
            match connection::probe_server(&server, expected_chain_id)? {
                connection::ProbeOutcome::Connected { session, latency } => match strategy {
                    ConnectionStrategy::FirstAvailable => return Ok(Self { session }),
                    ConnectionStrategy::LowestLatency => {
                        if best.as_ref().is_none_or(|(best, _)| latency < *best) {
                            best = Some((latency, session));
                        }
                    }
                },
                connection::ProbeOutcome::Failed(failure) => attempts.push(failure),
            }
        }

        match best {
            Some((_, session)) => Ok(Self { session }),
            None => Err(SwaplockApiError::AllServersFailed { attempts }),
        }
    }

    /// Measure connect latency for every server and return it sorted fastest-first.
    ///
    /// Probes are sequential. Unreachable servers are omitted from the report; a chain-id
    /// mismatch aborts immediately. Each probe opens and then closes a session, so this is a
    /// health check, not a connection — follow it with [`connect_with_strategy`](Self::connect_with_strategy).
    pub async fn probe_latencies<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
    ) -> Result<Vec<ServerLatency>, SwaplockApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let servers = servers.into_iter().map(Into::into).collect::<Vec<_>>();
        if servers.is_empty() {
            return Err(SwaplockApiError::MissingServers);
        }

        let mut latencies = Vec::new();
        for server in servers {
            if let connection::ProbeOutcome::Connected { session, latency } =
                connection::probe_server(&server, expected_chain_id)?
            {
                drop(session);
                latencies.push(ServerLatency { server, latency });
            }
        }

        latencies.sort_by_key(|entry| entry.latency);
        Ok(latencies)
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

    pub fn into_live(self) -> Result<SwaplockLiveApi, SwaplockApiError> {
        SwaplockLiveApi::from_session(self.session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_servers_error_is_actionable() {
        assert_eq!(
            SwaplockApiError::MissingServers.to_string(),
            "at least one Swaplock RPC server is required"
        );
    }

    #[test]
    fn chain_id_mismatch_error_includes_expected_and_actual_values() {
        let error = SwaplockApiError::ChainIdMismatch {
            mismatch: ChainIdMismatch {
                server: "wss://node.example".to_string(),
                expected: "expected".to_string(),
                actual: "actual".to_string(),
            },
        };

        let message = error.to_string();
        assert!(message.contains("expected"));
        assert!(message.contains("actual"));
    }

    #[test]
    fn asset_not_found_error_names_asset() {
        let error = SwaplockApiError::AssetNotFound {
            asset: "BTS".to_string(),
        };

        assert_eq!(error.to_string(), "asset `BTS` was not found");
    }
}
