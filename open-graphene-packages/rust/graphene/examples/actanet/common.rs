//! Shared by the examples: where to find an Actanet node.
//!
//! There is no public Actanet node yet, so the examples read the nodes from `ACTANET_RPC_SERVERS`
//! (comma-separated) and fall back to a local node.

/// The nodes to connect to.
pub fn servers() -> Vec<String> {
    std::env::var("ACTANET_RPC_SERVERS")
        .unwrap_or_else(|_| "ws://127.0.0.1:8090".to_string())
        .split(',')
        .map(|server| server.trim().to_string())
        .filter(|server| !server.is_empty())
        .collect()
}
