use open_graphene_transport::{GrapheneSession, TransportError, parse_chain_id};
use serde_json::json;
use thiserror::Error;

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

    #[error("all Swaplock RPC servers failed: {attempts:?}")]
    AllServersFailed { attempts: Vec<ServerConnectFailure> },

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
}

pub struct DatabaseApi<'session> {
    session: &'session mut GrapheneSession,
}

impl DatabaseApi<'_> {
    pub async fn get_chain_id(&mut self) -> Result<String, SwaplockApiError> {
        let value = self.session.database_call("get_chain_id", json!([]))?;
        Ok(parse_chain_id(value)?)
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
                server: "wss://example.invalid".to_string(),
                expected: "expected-chain".to_string(),
                actual: "actual-chain".to_string(),
            },
        };
        let message = error.to_string();

        assert!(message.contains("expected-chain"));
        assert!(message.contains("actual-chain"));
        assert!(message.contains("wss://example.invalid"));
    }
}
