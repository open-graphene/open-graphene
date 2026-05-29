mod database;
mod history;
mod network_broadcast;
mod operations;

use open_graphene_core::{AmountError, BalanceError, HeaderError, ObjectIdError};
use open_graphene_transport::{GrapheneSession, TransportError};
use thiserror::Error;

pub use network_broadcast::{BroadcastConfirmation, BroadcastReceipt, NetworkBroadcastApi};
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
    pub async fn connect<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
    ) -> Result<Self, SwaplockApiError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let servers = servers.into_iter().map(Into::into).collect::<Vec<_>>();
        if servers.is_empty() {
            return Err(SwaplockApiError::MissingServers);
        }

        let expected_chain_id = expected_chain_id.map(str::to_string);
        let mut attempts = Vec::new();
        for server in servers {
            match GrapheneSession::connect(&server) {
                Ok(session) => {
                    if let Some(expected) = expected_chain_id.as_deref() {
                        let actual = session.chain_id();
                        if actual != expected {
                            return Err(SwaplockApiError::ChainIdMismatch {
                                mismatch: ChainIdMismatch {
                                    server,
                                    expected: expected.to_string(),
                                    actual: actual.to_string(),
                                },
                            });
                        }
                    }
                    return Ok(Self { session });
                }
                Err(error) => attempts.push(ServerConnectFailure {
                    server,
                    error: error.to_string(),
                }),
            }
        }

        Err(SwaplockApiError::AllServersFailed { attempts })
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
