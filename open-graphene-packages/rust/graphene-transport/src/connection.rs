//! Picking which node to talk to: failover and latency-sorted selection.
//!
//! This is a ws/connection concern (which node, retrying, measuring connect latency), so it
//! lives in transport and is shared by every chain's API layer rather than copied per chain.

use std::time::{Duration, Instant};

use crate::{GrapheneSession, TransportError};

/// How [`GrapheneSession::connect_with_strategy`] picks among several RPC servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionStrategy {
    /// Connect to the first server that answers, in the order given (default).
    #[default]
    FirstAvailable,
    /// Probe every server, then keep the session with the lowest connect latency.
    LowestLatency,
}

/// How a session retries an idempotent call after the connection drops.
///
/// Applied only to read calls (database/history/crypto/orders), which are safe to repeat.
/// Mutating broadcasts are never retried automatically, since a resend could double-submit.
/// Set `max_retries` to 0 to disable auto-reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    /// How many times to reconnect-and-retry before giving up.
    pub max_retries: u32,
    /// Delay before the first retry; it doubles each attempt up to `backoff_max`.
    pub backoff_base: Duration,
    /// Ceiling on the per-attempt backoff delay.
    pub backoff_max: Duration,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_base: Duration::from_millis(200),
            backoff_max: Duration::from_secs(5),
        }
    }
}

impl ReconnectPolicy {
    /// Auto-reconnect turned off: calls fail on the first connection error.
    pub fn disabled() -> Self {
        Self {
            max_retries: 0,
            ..Self::default()
        }
    }

    /// Backoff delay before retry `attempt` (1-based): `backoff_base * 2^(attempt-1)`, capped.
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(31);
        let scaled = self.backoff_base.saturating_mul(1u32 << shift);
        scaled.min(self.backoff_max)
    }
}

/// Whether `error` is a dropped/broken connection (worth reconnecting) rather than an application
/// or protocol error (which a reconnect would not fix).
pub fn is_connection_error(error: &TransportError) -> bool {
    matches!(
        error,
        TransportError::WebSocket(_) | TransportError::Io(_) | TransportError::ConnectionClosed
    )
}

/// Measured connect latency for a single RPC server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerLatency {
    pub server: String,
    pub latency: Duration,
}

/// A recoverable failure to connect to one server (collected for `AllServersFailed`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConnectFailure {
    pub server: String,
    pub error: String,
}

/// A connected server reported a chain id different from the one expected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainIdMismatch {
    pub server: String,
    pub expected: String,
    pub actual: String,
}

/// A connected session (with its measured latency) or a recoverable connect failure.
///
/// `Connected` is large because it holds a live session, but the value is matched and unwrapped
/// at once at the call site and never stored, so boxing would only add a wasted allocation on
/// the hot connect path.
#[allow(clippy::large_enum_variant)]
enum ProbeOutcome {
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
fn probe_server(
    server: &str,
    expected_chain_id: Option<&str>,
) -> Result<ProbeOutcome, TransportError> {
    let started = Instant::now();
    match GrapheneSession::connect(server) {
        Ok(session) => {
            let latency = started.elapsed();
            if let Some(expected) = expected_chain_id {
                let actual = session.chain_id();
                if actual != expected {
                    return Err(TransportError::ChainIdMismatch {
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

impl GrapheneSession {
    /// Connect to one of `servers`, choosing which according to `strategy`.
    ///
    /// - [`ConnectionStrategy::FirstAvailable`] returns as soon as a server answers.
    /// - [`ConnectionStrategy::LowestLatency`] probes every server and keeps the fastest session.
    ///
    /// Unreachable servers are collected and, if none succeed, reported as
    /// [`TransportError::AllServersFailed`]. A chain-id mismatch aborts immediately.
    pub fn connect_with_strategy<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
        strategy: ConnectionStrategy,
    ) -> Result<Self, TransportError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let servers = servers.into_iter().map(Into::into).collect::<Vec<_>>();
        if servers.is_empty() {
            return Err(TransportError::MissingServers);
        }

        let mut attempts = Vec::new();
        let mut best: Option<(Duration, GrapheneSession)> = None;
        for server in servers {
            match probe_server(&server, expected_chain_id)? {
                ProbeOutcome::Connected { session, latency } => match strategy {
                    ConnectionStrategy::FirstAvailable => return Ok(session),
                    ConnectionStrategy::LowestLatency => {
                        if best.as_ref().is_none_or(|(best, _)| latency < *best) {
                            best = Some((latency, session));
                        }
                    }
                },
                ProbeOutcome::Failed(failure) => attempts.push(failure),
            }
        }

        match best {
            Some((_, session)) => Ok(session),
            None => Err(TransportError::AllServersFailed { attempts }),
        }
    }

    /// Rank every server by how fast it completes a connection, fastest first.
    ///
    /// A health check, not a live connection: each server is opened and then closed. Probes run
    /// sequentially. Unreachable servers drop out of the report; a wrong chain id aborts at once.
    /// Follow with [`connect_with_strategy`](Self::connect_with_strategy) to actually connect.
    pub fn probe_latencies<I, S>(
        servers: I,
        expected_chain_id: Option<&str>,
    ) -> Result<Vec<ServerLatency>, TransportError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let servers = servers.into_iter().map(Into::into).collect::<Vec<_>>();
        if servers.is_empty() {
            return Err(TransportError::MissingServers);
        }

        let mut latencies = Vec::new();
        for server in servers {
            if let ProbeOutcome::Connected { session, latency } =
                probe_server(&server, expected_chain_id)?
            {
                drop(session);
                latencies.push(ServerLatency { server, latency });
            }
        }

        latencies.sort_by_key(|entry| entry.latency);
        Ok(latencies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_caps() {
        let policy = ReconnectPolicy {
            max_retries: 10,
            backoff_base: Duration::from_millis(100),
            backoff_max: Duration::from_millis(500),
        };
        assert_eq!(policy.backoff_delay(1), Duration::from_millis(100));
        assert_eq!(policy.backoff_delay(2), Duration::from_millis(200));
        assert_eq!(policy.backoff_delay(3), Duration::from_millis(400));
        assert_eq!(policy.backoff_delay(4), Duration::from_millis(500)); // capped
        assert_eq!(policy.backoff_delay(99), Duration::from_millis(500)); // no overflow
    }

    #[test]
    fn disabled_policy_does_not_retry() {
        assert_eq!(ReconnectPolicy::disabled().max_retries, 0);
    }

    #[test]
    fn only_connection_errors_trigger_reconnect() {
        assert!(is_connection_error(&TransportError::ConnectionClosed));
        assert!(is_connection_error(&TransportError::WebSocket(
            "boom".into()
        )));
        assert!(!is_connection_error(&TransportError::RpcError {
            id: 1,
            error: serde_json::json!({}),
        }));
        assert!(!is_connection_error(&TransportError::MissingApi {
            name: "history"
        }));
    }

    #[test]
    fn first_available_is_the_default_strategy() {
        assert_eq!(
            ConnectionStrategy::default(),
            ConnectionStrategy::FirstAvailable
        );
    }

    #[test]
    fn server_latency_sorts_ascending_by_duration() {
        let mut report = [
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

    #[test]
    fn missing_servers_error_is_actionable() {
        assert_eq!(
            TransportError::MissingServers.to_string(),
            "at least one RPC server is required"
        );
    }

    #[test]
    fn chain_id_mismatch_error_includes_expected_and_actual_values() {
        let error = TransportError::ChainIdMismatch {
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
}
