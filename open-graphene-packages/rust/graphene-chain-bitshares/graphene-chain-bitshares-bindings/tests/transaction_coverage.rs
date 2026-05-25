use graphene_chain_bitshares_bindings::generated::FcSerialize;
use graphene_chain_bitshares_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_bitshares_bindings::generated::operations::TransferOperation;
use graphene_chain_bitshares_bindings::generated::static_variants::{FutureExtensions, Operation};
use graphene_chain_bitshares_bindings::generated::types::{
    Asset, Signature, SignedTransaction, Transaction,
};

fn transfer_operation() -> Operation {
    Operation::TransferOperation(Box::new(TransferOperation {
        fee: Asset {
            amount: 200_000,
            asset_id: AssetId("1.3.0".to_string()),
        },
        from: AccountId("1.2.100".to_string()),
        to: AccountId("1.2.0".to_string()),
        amount: Asset {
            amount: 100_000,
            asset_id: AssetId("1.3.0".to_string()),
        },
        memo: None,
        extensions: FutureExtensions::VoidT(Box::new(())),
    }))
}

#[test]
fn bitshares_transaction_serializes_transfer_operation_vector() {
    let transaction = Transaction {
        ref_block_num: 2,
        ref_block_prefix: 3,
        expiration: "2026-05-25T12:01:00".to_string(),
        operations: vec![transfer_operation()],
        extensions: FutureExtensions::VoidT(Box::new(())),
    };

    let bytes = transaction.to_fc_bytes().unwrap();

    assert_eq!(&bytes[0..2], &2u16.to_le_bytes());
    assert_eq!(&bytes[2..6], &3u32.to_le_bytes());
    assert!(bytes.len() > 6);
}

#[test]
fn bitshares_signed_transaction_serializes_signature_vector() {
    let signed_transaction = SignedTransaction {
        ref_block_num: 2,
        ref_block_prefix: 3,
        expiration: "2026-05-25T12:01:00".to_string(),
        operations: vec![transfer_operation()],
        extensions: FutureExtensions::VoidT(Box::new(())),
        signatures: vec![Signature(vec![0x1f; 65])],
    };

    let bytes = signed_transaction.to_fc_bytes().unwrap();

    assert!(bytes.ends_with(&[0x1f; 65]));
}
