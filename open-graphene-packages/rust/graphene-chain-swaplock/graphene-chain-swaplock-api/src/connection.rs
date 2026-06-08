use std::time::{Duration, Instant};

use open_graphene_transport::GrapheneSession;

use crate::{ChainIdMismatch, ServerConnectFailure, SwaplockApiError};

/// How [`SwaplockApi::connect`](crate::SwaplockApi::connect) picks among several RPC servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionStrategy {
    /// Connect to the first server that answers, in the order given (default).
    #[default]
    FirstAvailable,
    /// Probe every server, then keep the session with the lowest connect latency.
    LowestLatency,
}

/// Measured connect latency for a single RPC server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerLatency {
    pub server: String,
    pub latency: Duration,
}

/// Result of probing one server: a live session (with its latency) or a recoverable failure.
///
/// The `Connected` variant is large because it carries a live session, but a `ProbeOutcome` is
/// always matched and unwrapped immediately at the call site — it is never stored or collected —
/// so boxing would only add a pointless allocation on the hot connect path.
#[allow(clippy::large_enum_variant)]
pub(crate) enum ProbeOutcome {
    Connected {
        session: GrapheneSession,
        latency: Duration,
    },
    Failed(ServerConnectFailure),
}

/// Connect to a single server, measuring how long the full handshake takes.
///
/// A failed connection is returned as [`ProbeOutcome::Failed`] so callers can try the next server.
/// A chain-id mismatch is a configuration error and is surfaced immediately as `Err`.
pub(crate) fn probe_server(
    server: &str,
    expected_chain_id: Option<&str>,
) -> Result<ProbeOutcome, SwaplockApiError> {
    let started = Instant::now();
    match GrapheneSession::connect(server) {
        Ok(session) => {
            let latency = started.elapsed();
            if let Some(expected) = expected_chain_id {
                let actual = session.chain_id();
                if actual != expected {
                    return Err(SwaplockApiError::ChainIdMismatch {
                        mismatch: ChainIdMismatch {
                            server: server.to_string(),
                            expected: expected.to_string(),
                            actual: actual.to_string(),
                        },
                    });
                }
            }
            Ok(ProbeOutcome::Connected { session, latency })
        }
        Err(error) => Ok(ProbeOutcome::Failed(ServerConnectFailure {
            server: server.to_string(),
            error: error.to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_available_is_the_default_strategy() {
        assert_eq!(ConnectionStrategy::default(), ConnectionStrategy::FirstAvailable);
    }

    #[test]
    fn server_latency_sorts_ascending_by_duration() {
        let mut report = vec![
            ServerLatency {
                server: "slow".to_string(),
                latency: Duration::from_millis(300),
            },
            ServerLatency {
                server: "fast".to_string(),
                latency: Duration::from_millis(50),
            },
            ServerLatency {
                server: "mid".to_string(),
                latency: Duration::from_millis(120),
            },
        ];

        report.sort_by_key(|entry| entry.latency);

        let order: Vec<&str> = report.iter().map(|entry| entry.server.as_str()).collect();
        assert_eq!(order, ["fast", "mid", "slow"]);
    }
}
