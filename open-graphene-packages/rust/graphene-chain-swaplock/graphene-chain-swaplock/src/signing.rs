use std::error::Error;

use graphene_chain_swaplock_bindings::generated::fc::{
    decode_public_key, is_graphene_canonical_compact_signature, verify_compact_signature_public_key,
};
use graphene_chain_swaplock_bindings::generated::ids::PUBLIC_KEY_PREFIX;
use graphene_chain_swaplock_bindings::generated::types::{SignedTransaction, Transaction};

pub fn sign_transaction_checked(
    transaction: &Transaction,
    wif: &str,
    expected_public_key: &str,
) -> Result<SignedTransaction, Box<dyn Error>> {
    let signed_transaction = transaction.signed_with_wif(wif)?;
    let signature = signed_transaction
        .signatures
        .first()
        .ok_or("signed transaction has no signature")?;
    if !is_graphene_canonical_compact_signature(&signature.0) {
        return Err("signature is not Graphene canonical".into());
    }
    let matches_public_key = verify_compact_signature_public_key(
        transaction.signature_digest_bytes()?,
        &signature.0,
        decode_public_key(expected_public_key, Some(PUBLIC_KEY_PREFIX))?,
    )?;
    if !matches_public_key {
        return Err("signature public key verification failed".into());
    }
    Ok(signed_transaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::transfer::{build_transfer_transaction, TransferTransactionInput};

    const FIXTURE_WIF: &str = "5J4eFhjREJA7hKG6KcvHofHMXyGQZCDpQE463PAaKo9xXY6UDPq";
    const FIXTURE_PUBLIC_KEY: &str = "BTS7jDPoMwyjVH5obFmqzFNp4Ffp7G2nvC7FKFkrMBpo7Sy4uq5Mj";
    const WRONG_PUBLIC_KEY: &str = "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV";

    #[test]
    fn signs_when_expected_public_key_matches() {
        let signed =
            sign_transaction_checked(&fixture_transaction(), FIXTURE_WIF, FIXTURE_PUBLIC_KEY)
                .expect("fixture key signs transaction");

        assert_eq!(signed.signatures.len(), 1);
        assert!(is_graphene_canonical_compact_signature(
            &signed.signatures[0].0
        ));
    }

    #[test]
    fn rejects_when_expected_public_key_does_not_match() {
        let err = sign_transaction_checked(&fixture_transaction(), FIXTURE_WIF, WRONG_PUBLIC_KEY)
            .expect_err("wrong public key fails verification");

        assert_eq!(err.to_string(), "signature public key verification failed");
    }

    #[test]
    fn invalid_wif_error_does_not_echo_secret_like_input() {
        let secret_like_value = "not-a-wif-secret-like-value";
        let err = sign_transaction_checked(
            &fixture_transaction(),
            secret_like_value,
            FIXTURE_PUBLIC_KEY,
        )
        .expect_err("invalid WIF fails");

        assert!(!err.to_string().contains(secret_like_value));
    }

    fn fixture_transaction() -> Transaction {
        build_transfer_transaction(TransferTransactionInput {
            ref_block_num: 1,
            ref_block_prefix: 2,
            expiration: "2026-01-01T00:00:00".to_string(),
            from_id: "1.2.100".to_string(),
            to_id: "1.2.101".to_string(),
            asset_id: "1.3.0".to_string(),
            amount: 1,
            fee_amount: 0,
            fee_asset_id: "1.3.0".to_string(),
        })
    }
}
