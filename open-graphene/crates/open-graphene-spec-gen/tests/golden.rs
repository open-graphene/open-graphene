//! Full-pipeline golden test: runs config -> discover -> extract -> resolve ->
//! generate on the committed fixture chain tree and compares the emitted JSON
//! against a committed snapshot.
//!
//! To bless a new snapshot after an intentional generator change:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test -p open-graphene-spec-gen --test golden
//! ```

use std::fs;
use std::path::PathBuf;

use open_graphene_spec_gen::{
    build_protocol, discover_sources, extract_source_facts, load_config, resolve_rpc_methods,
    validate_protocol,
};

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

#[test]
fn fixture_chain_spec_matches_golden_snapshot() {
    let config_path = fixture_path("open-graphene.toml");
    let config = load_config(&config_path).expect("load fixture config");
    let source_set = discover_sources(&config_path, &config.source).expect("discover sources");
    let facts = extract_source_facts(&source_set).expect("extract source facts");
    let rpc_resolution = resolve_rpc_methods(&config, &facts);

    let extract_diagnostics: Vec<String> = facts
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{diagnostic:?}"))
        .collect();
    let resolve_diagnostics: Vec<String> = rpc_resolution
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{diagnostic:?}"))
        .collect();

    let build = build_protocol(&config, &facts, rpc_resolution.methods);
    let protocol = build.protocol;
    let mut rendered = serde_json::to_string_pretty(&protocol).expect("serialize fixture protocol");
    rendered.push('\n');

    // Diagnostics are part of the observable pipeline behavior: snapshot them
    // together with the spec so silently-dropped data shows up in review.
    let mut snapshot = String::new();
    snapshot.push_str("// extract diagnostics:\n");
    for line in &extract_diagnostics {
        snapshot.push_str(&format!("//   {line}\n"));
    }
    snapshot.push_str("// resolve diagnostics:\n");
    for line in &resolve_diagnostics {
        snapshot.push_str(&format!("//   {line}\n"));
    }
    snapshot.push_str("// build diagnostics:\n");
    for line in &build.diagnostics {
        snapshot.push_str(&format!("//   {line}\n"));
    }
    snapshot.push_str("// validation issues:\n");
    for issue in validate_protocol(&protocol) {
        snapshot.push_str(&format!("//   {issue}\n"));
    }
    snapshot.push_str(&rendered);

    let golden_path = fixture_path("fixture-chain.golden.jsonc");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&golden_path, &snapshot).expect("write golden snapshot");
        return;
    }

    let golden = fs::read_to_string(&golden_path).unwrap_or_else(|error| {
        panic!(
            "missing golden snapshot {} ({error}); run UPDATE_GOLDEN=1 cargo test -p open-graphene-spec-gen --test golden",
            golden_path.display()
        )
    });
    pretty_assertions::assert_eq!(
        golden,
        snapshot,
        "generated fixture spec drifted from the golden snapshot; if the change is intentional, bless it with UPDATE_GOLDEN=1"
    );
}
