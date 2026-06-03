//! Low-level `crypto` API calls.
//!
//! This layer only shapes JSON-RPC params and forwards them; the typed
//! commitment / blinding-factor / proof domain lives in the higher SDK crate.
//! Blob arguments arrive already encoded as JSON (hex strings, as `fc` expects).

use serde_json::{Value, json};

use crate::{GrapheneSession, TransportError};

pub fn blind(
    session: &mut GrapheneSession,
    blinding_factor: Value,
    value: u64,
) -> Result<Value, TransportError> {
    session.crypto_call("blind", blind_params(blinding_factor, value))
}

fn blind_params(blinding_factor: Value, value: u64) -> Value {
    json!([blinding_factor, value])
}

pub fn blind_sum(
    session: &mut GrapheneSession,
    blinds_in: Value,
    non_neg: u32,
) -> Result<Value, TransportError> {
    session.crypto_call("blind_sum", blind_sum_params(blinds_in, non_neg))
}

fn blind_sum_params(blinds_in: Value, non_neg: u32) -> Value {
    json!([blinds_in, non_neg])
}

pub fn verify_sum(
    session: &mut GrapheneSession,
    commits_in: Value,
    neg_commits_in: Value,
    excess: i64,
) -> Result<Value, TransportError> {
    session.crypto_call(
        "verify_sum",
        verify_sum_params(commits_in, neg_commits_in, excess),
    )
}

fn verify_sum_params(commits_in: Value, neg_commits_in: Value, excess: i64) -> Value {
    json!([commits_in, neg_commits_in, excess])
}

pub fn verify_range(
    session: &mut GrapheneSession,
    commit: Value,
    proof: Value,
) -> Result<Value, TransportError> {
    session.crypto_call("verify_range", verify_range_params(commit, proof))
}

fn verify_range_params(commit: Value, proof: Value) -> Value {
    json!([commit, proof])
}

#[allow(clippy::too_many_arguments)]
pub fn range_proof_sign(
    session: &mut GrapheneSession,
    min_value: u64,
    commit: Value,
    commit_blind: Value,
    nonce: Value,
    base10_exp: i8,
    min_bits: u8,
    actual_value: u64,
) -> Result<Value, TransportError> {
    session.crypto_call(
        "range_proof_sign",
        range_proof_sign_params(
            min_value,
            commit,
            commit_blind,
            nonce,
            base10_exp,
            min_bits,
            actual_value,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn range_proof_sign_params(
    min_value: u64,
    commit: Value,
    commit_blind: Value,
    nonce: Value,
    base10_exp: i8,
    min_bits: u8,
    actual_value: u64,
) -> Value {
    json!([
        min_value,
        commit,
        commit_blind,
        nonce,
        base10_exp,
        min_bits,
        actual_value
    ])
}

pub fn verify_range_proof_rewind(
    session: &mut GrapheneSession,
    nonce: Value,
    commit: Value,
    proof: Value,
) -> Result<Value, TransportError> {
    session.crypto_call(
        "verify_range_proof_rewind",
        verify_range_proof_rewind_params(nonce, commit, proof),
    )
}

fn verify_range_proof_rewind_params(nonce: Value, commit: Value, proof: Value) -> Value {
    json!([nonce, commit, proof])
}

pub fn range_get_info(
    session: &mut GrapheneSession,
    proof: Value,
) -> Result<Value, TransportError> {
    session.crypto_call("range_get_info", range_get_info_params(proof))
}

fn range_get_info_params(proof: Value) -> Value {
    json!([proof])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blind_params_pass_hex_factor_then_value() {
        assert_eq!(blind_params(json!("0a0b"), 1_000), json!(["0a0b", 1_000]));
    }

    #[test]
    fn blind_sum_params_pass_factor_array_then_non_neg() {
        assert_eq!(
            blind_sum_params(json!(["1111", "2222"]), 2),
            json!([["1111", "2222"], 2])
        );
    }

    #[test]
    fn verify_sum_params_keep_commits_negatives_and_excess() {
        assert_eq!(
            verify_sum_params(json!(["aa"]), json!(["bb"]), -3),
            json!([["aa"], ["bb"], -3])
        );
    }

    #[test]
    fn verify_range_params_pass_commit_then_proof() {
        assert_eq!(
            verify_range_params(json!("aa"), json!("bb")),
            json!(["aa", "bb"])
        );
    }

    #[test]
    fn range_proof_sign_params_match_node_argument_order() {
        assert_eq!(
            range_proof_sign_params(0, json!("cc"), json!("dd"), json!("ee"), 0, 32, 1_000),
            json!([0, "cc", "dd", "ee", 0, 32, 1_000])
        );
    }

    #[test]
    fn verify_range_proof_rewind_params_pass_nonce_commit_proof() {
        assert_eq!(
            verify_range_proof_rewind_params(json!("aa"), json!("bb"), json!("cc")),
            json!(["aa", "bb", "cc"])
        );
    }

    #[test]
    fn range_get_info_params_wrap_proof() {
        assert_eq!(range_get_info_params(json!("ff")), json!(["ff"]));
    }
}
