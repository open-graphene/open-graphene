use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::config::ChainConfig;
use crate::error::{Result, SpecGenError};

/// The chain ID a Graphene node derives from a genesis file: the SHA-256 of its raw bytes.
///
/// `witness_node --genesis-json` hashes the file contents exactly as read
/// (`libraries/app/application.cpp`), and the build hashes the embedded genesis the same way
/// (`libraries/egenesis/embed_genesis.cmake`). Any byte changes it, whitespace included.
pub fn chain_id_from_genesis(genesis: &[u8]) -> String {
    Sha256::digest(genesis)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The chain ID to pin in the spec.
///
/// With `chain.genesis` set it is computed from that file, read from `chain_repo`, and a
/// `chain.chain_id` given as well must agree with it. Otherwise it is `chain.chain_id` as
/// configured, which may be absent.
pub fn resolve_chain_id(chain: &ChainConfig, chain_repo: &Path) -> Result<Option<String>> {
    let Some(genesis) = &chain.genesis else {
        return Ok(chain.chain_id.clone());
    };
    let path = chain_repo.join(genesis);
    let bytes = fs::read(&path).map_err(|source| SpecGenError::ReadGenesis {
        path: path.clone(),
        source,
    })?;
    let computed = chain_id_from_genesis(&bytes);
    if let Some(configured) = &chain.chain_id
        && !configured.eq_ignore_ascii_case(&computed)
    {
        return Err(SpecGenError::ChainIdMismatch {
            path,
            computed,
            configured: configured.clone(),
        });
    }
    Ok(Some(computed))
}
