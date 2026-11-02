//! The chain ID computed from a genesis file, as `[chain] genesis` configures it.

use std::path::PathBuf;

use open_graphene_spec_gen::{ChainConfig, SpecGenError, chain_id_from_genesis, resolve_chain_id};

/// `sha256sum tests/fixtures/genesis/genesis.json`
const FIXTURE_CHAIN_ID: &str = "e242c4077595a2039f35477eee422b408a7fcb5cbd0bacddbaffd1dc8b5ed0a5";

fn fixture_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/genesis")
}

fn chain(chain_id: Option<&str>, genesis: Option<&str>) -> ChainConfig {
    ChainConfig {
        id: "fixture".to_string(),
        public_key_prefix: "TEST".to_string(),
        chain_id: chain_id.map(str::to_string),
        genesis: genesis.map(str::to_string),
    }
}

#[test]
fn chain_id_is_the_sha256_of_the_raw_genesis_bytes() {
    assert_eq!(
        chain_id_from_genesis(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // A trailing newline is part of the file, so it is part of the chain ID.
    assert_ne!(chain_id_from_genesis(b"{}"), chain_id_from_genesis(b"{}\n"));
}

#[test]
fn genesis_file_sets_the_chain_id() {
    let resolved = resolve_chain_id(&chain(None, Some("genesis.json")), &fixture_repo())
        .expect("resolve chain id");
    assert_eq!(resolved.as_deref(), Some(FIXTURE_CHAIN_ID));
}

#[test]
fn configured_chain_id_must_agree_with_the_genesis() {
    let agreeing = chain(Some(&FIXTURE_CHAIN_ID.to_uppercase()), Some("genesis.json"));
    assert_eq!(
        resolve_chain_id(&agreeing, &fixture_repo())
            .expect("resolve chain id")
            .as_deref(),
        Some(FIXTURE_CHAIN_ID)
    );

    let other = "00".repeat(32);
    let conflicting = chain(Some(&other), Some("genesis.json"));
    assert!(matches!(
        resolve_chain_id(&conflicting, &fixture_repo()),
        Err(SpecGenError::ChainIdMismatch { computed, configured, .. })
            if computed == FIXTURE_CHAIN_ID && configured == other
    ));
}

#[test]
fn without_a_genesis_the_configured_chain_id_is_kept() {
    let pinned = "11".repeat(32);
    assert_eq!(
        resolve_chain_id(&chain(Some(&pinned), None), &fixture_repo())
            .expect("resolve chain id")
            .as_deref(),
        Some(pinned.as_str())
    );
    assert_eq!(
        resolve_chain_id(&chain(None, None), &fixture_repo()).expect("resolve chain id"),
        None
    );
}

#[test]
fn missing_genesis_file_is_an_error() {
    assert!(matches!(
        resolve_chain_id(&chain(None, Some("absent.json")), &fixture_repo()),
        Err(SpecGenError::ReadGenesis { .. })
    ));
}
