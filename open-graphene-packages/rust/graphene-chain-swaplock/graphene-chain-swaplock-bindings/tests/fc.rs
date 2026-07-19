use graphene_chain_swaplock_bindings::generated::{
    AccountCreateOperation, AccountCreateOperationExt, AccountId, AccountNameEqLitPredicate,
    AccountOptions, ArgumentType, AssertOperation, Asset, AssetId, AssetSymbolEqLitPredicate,
    AssetUpdateFeedProducersOperation, Authority, BlockIdPredicate, BurnWorkerInitializer,
    CddVestingPolicyInitializer, ChainParameters, ChainParametersExt,
    CommitteeMemberUpdateGlobalParametersOperation,
    CommitteeMemberUpdateGlobalParametersOperationFeeParamsT, CreateTakeProfitOrderAction,
    CreditOfferCreateOperation, CreditOfferId, CreditOfferUpdateOperation,
    CustomAuthorityCreateOperation, CustomAuthorityId, CustomAuthorityUpdateOperation,
    CustomOperation, FcSerialize, FcSerializeError, FeeParameters, FeeSchedule, HtlcHash, HtlcId,
    HtlcRefundOperation, InstantVestingPolicyInitializer, LimitOrderAutoAction,
    LimitOrderCancelOperation, LimitOrderCreateOperation, LimitOrderId, LimitOrderUpdateOperation,
    LinearVestingPolicyInitializer, MemoData, NoSpecialAuthority, OpWrapper, Operation, Predicate,
    Price, ProposalCreateOperation, RefundWorkerInitializer, Restriction, Signature,
    SignedTransaction, SpecialAuthority, TopHoldersSpecialAuthority, Transaction,
    TransferOperation, TransferOperationFeeParamsT, VestingBalanceCreateOperation,
    VestingBalanceWorkerInitializer, VestingPolicyInitializer, WithdrawPermissionCreateOperation,
    WorkerCreateOperation, WorkerInitializer, sha256_bytes,
};

fn sample_transfer_operation() -> TransferOperation {
    TransferOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        from: AccountId("1.2.1".to_string()),
        to: AccountId("1.2.2".to_string()),
        amount: Asset {
            amount: 100_000,
            asset_id: AssetId("1.3.0".to_string()),
        },
        memo: None,
        extensions: vec![],
    }
}

fn expected_transfer_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee: amount 0 + asset instance 0
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // from account instance 1, to account instance 2
    bytes.extend_from_slice(&[1, 2]);
    // amount: amount 100_000 + asset instance 0
    bytes.extend_from_slice(&[0xa0, 0x86, 0x01, 0, 0, 0, 0, 0, 0]);
    // memo None, extensions tag 0
    bytes.extend_from_slice(&[0, 0]);
    bytes
}

fn sample_proposal_create_operation() -> ProposalCreateOperation {
    ProposalCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        fee_paying_account: AccountId("1.2.1".to_string()),
        expiration_time: "1970-01-01T00:00:03".to_string(),
        proposed_ops: vec![OpWrapper {
            op: Operation::TransferOperation(Box::new(sample_transfer_operation())),
        }],
        review_period_seconds: Some(60),
        extensions: vec![],
    }
}

fn expected_proposal_create_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, fee_paying_account instance 1, expiration time 3
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 3, 0, 0, 0]);
    // proposed_ops length 1, op_wrapper.op = Operation::TransferOperation tag 0 + payload
    bytes.extend_from_slice(&[1, 0]);
    bytes.extend(expected_transfer_payload());
    // review_period_seconds Some(60), extensions tag 0
    bytes.extend_from_slice(&[1, 60, 0, 0, 0, 0]);
    bytes
}

fn sample_transaction_with_operations(operations: Vec<Operation>) -> Transaction {
    Transaction {
        ref_block_num: 1,
        ref_block_prefix: 2,
        expiration: "1970-01-01T00:00:03".to_string(),
        operations,
        extensions: vec![],
    }
}

fn sample_signed_transaction_with_operations(
    operations: Vec<Operation>,
    signatures: Vec<Signature>,
) -> SignedTransaction {
    SignedTransaction {
        ref_block_num: 1,
        ref_block_prefix: 2,
        expiration: "1970-01-01T00:00:03".to_string(),
        operations,
        extensions: vec![],
        signatures,
    }
}

fn expected_transaction_payload(operation_payloads: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    // ref_block_num 1, ref_block_prefix 2, expiration 3
    bytes.extend_from_slice(&[1, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
    bytes.push(operation_payloads.len() as u8);
    for payload in operation_payloads {
        bytes.extend(payload);
    }
    // extensions tag 0
    bytes.push(0);
    bytes
}

fn sample_signature_bytes() -> Vec<u8> {
    let mut signature = Vec::with_capacity(65);
    signature.push(31);
    signature.extend(1u8..=64);
    signature
}

fn bitsharesjs_signature_vector_bytes() -> Vec<u8> {
    decode_hex(
        "1f4a8b0c4b54ffd78ee503c2e9112932b2caa726086aa1acb72cc550e1fea0407e4765ad8766314c4e8eb2b42017ac1048ebece448639f63d8082a1e893009a874",
    )
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "hex string must have even length");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (decode_hex_nibble(pair[0]) << 4) | decode_hex_nibble(pair[1]))
        .collect()
}

fn decode_hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("invalid hex nibble"),
    }
}

fn expected_signed_transaction_payload(
    operation_payloads: &[Vec<u8>],
    signatures: &[Vec<u8>],
) -> Vec<u8> {
    let mut bytes = expected_transaction_payload(operation_payloads);
    bytes.push(signatures.len() as u8);
    for signature in signatures {
        bytes.extend(signature);
    }
    bytes
}

fn expected_swaplock_chain_id_bytes() -> Vec<u8> {
    vec![
        0x22, 0x67, 0xf6, 0x94, 0xd9, 0x6b, 0x7f, 0xfd, 0xcb, 0xa1, 0xa9, 0x8c, 0x63, 0xc0, 0x9e,
        0x72, 0x0a, 0x18, 0xa8, 0x5a, 0xd3, 0x49, 0x54, 0xe2, 0x99, 0xc6, 0x6d, 0x5a, 0x42, 0x23,
        0x40, 0x98,
    ]
}

fn sample_limit_order_create_operation() -> LimitOrderCreateOperation {
    LimitOrderCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        seller: AccountId("1.2.1".to_string()),
        amount_to_sell: Asset {
            amount: 1,
            asset_id: AssetId("1.3.0".to_string()),
        },
        min_to_receive: Asset {
            amount: 2,
            asset_id: AssetId("1.3.0".to_string()),
        },
        expiration: "2026-05-25T12:01:00".to_string(),
        fill_or_kill: true,
        extensions: vec![],
    }
}

fn sample_limit_order_cancel_operation() -> LimitOrderCancelOperation {
    LimitOrderCancelOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        fee_paying_account: AccountId("1.2.1".to_string()),
        order: LimitOrderId("1.7.1".to_string()),
        extensions: vec![],
    }
}

fn sample_limit_order_update_operation() -> LimitOrderUpdateOperation {
    LimitOrderUpdateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        seller: AccountId("1.2.1".to_string()),
        order: LimitOrderId("1.7.1".to_string()),
        new_price: None,
        delta_amount_to_sell: None,
        new_expiration: None,
        on_fill: Some(vec![LimitOrderAutoAction::CreateTakeProfitOrderAction(
            Box::new(CreateTakeProfitOrderAction {
                fee_asset_id: AssetId("1.3.0".to_string()),
                spread_percent: 25,
                size_percent: 50,
                expiration_seconds: 3_600,
                repeat: true,
                extensions: vec![],
            }),
        )]),
        extensions: vec![],
    }
}

fn sample_price() -> Price {
    Price {
        base: Asset {
            amount: 1,
            asset_id: AssetId("1.3.0".to_string()),
        },
        quote: Asset {
            amount: 2,
            asset_id: AssetId("1.3.1".to_string()),
        },
    }
}

fn sample_credit_offer_create_operation() -> CreditOfferCreateOperation {
    CreditOfferCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        owner_account: AccountId("1.2.1".to_string()),
        asset_type: AssetId("1.3.0".to_string()),
        balance: 1_000,
        fee_rate: 25,
        max_duration_seconds: 3_600,
        min_deal_amount: 10,
        enabled: true,
        auto_disable_time: "2020-01-01T00:00:00".to_string(),
        acceptable_collateral: vec![(AssetId("1.3.0".to_string()), sample_price())],
        acceptable_borrowers: vec![(AccountId("1.2.1".to_string()), 99)],
        extensions: vec![],
    }
}

fn sample_credit_offer_update_operation() -> CreditOfferUpdateOperation {
    CreditOfferUpdateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        owner_account: AccountId("1.2.1".to_string()),
        offer_id: CreditOfferId("1.21.1".to_string()),
        delta_amount: None,
        fee_rate: None,
        max_duration_seconds: None,
        min_deal_amount: None,
        enabled: None,
        auto_disable_time: None,
        acceptable_collateral: Some(vec![(AssetId("1.3.0".to_string()), sample_price())]),
        acceptable_borrowers: Some(vec![(AccountId("1.2.1".to_string()), 99)]),
        extensions: vec![],
    }
}

fn sample_fee_schedule() -> FeeSchedule {
    FeeSchedule {
        parameters: vec![
            FeeParameters::TransferOperationFeeParamsT(Box::new(TransferOperationFeeParamsT {
                fee: 5,
                price_per_kbyte: 6,
            })),
            FeeParameters::CommitteeMemberUpdateGlobalParametersOperationFeeParamsT(Box::new(
                CommitteeMemberUpdateGlobalParametersOperationFeeParamsT { fee: 7 },
            )),
        ],
        scale: 100,
    }
}

fn sample_chain_parameters() -> ChainParameters {
    ChainParameters {
        current_fees: sample_fee_schedule(),
        block_interval: 0,
        maintenance_interval: 0,
        maintenance_skip_slots: 0,
        committee_proposal_review_period: 0,
        maximum_transaction_size: 0,
        maximum_block_size: 0,
        maximum_time_until_expiration: 0,
        maximum_proposal_lifetime: 0,
        maximum_asset_whitelist_authorities: 0,
        maximum_asset_feed_publishers: 0,
        maximum_witness_count: 0,
        maximum_committee_count: 0,
        maximum_authority_membership: 0,
        reserve_percent_of_fee: 0,
        network_percent_of_fee: 0,
        lifetime_referrer_percent_of_fee: 0,
        cashback_vesting_period_seconds: 0,
        cashback_vesting_threshold: 0,
        count_non_member_votes: false,
        allow_non_member_whitelists: false,
        witness_pay_per_block: 0,
        worker_budget_per_day: 0,
        max_predicate_opcode: 0,
        fee_liquidation_threshold: 0,
        accounts_per_fee_scale: 0,
        account_fee_scale_bitshifts: 0,
        max_authority_depth: 0,
        extensions: ChainParametersExt {
            updatable_htlc_options: None,
            custom_authority_options: None,
            market_fee_network_percent: None,
            maker_fee_discount_percent: None,
        },
    }
}

fn sample_committee_member_update_global_parameters_operation()
-> CommitteeMemberUpdateGlobalParametersOperation {
    CommitteeMemberUpdateGlobalParametersOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        new_parameters: sample_chain_parameters(),
    }
}

fn sample_restriction() -> Restriction {
    Restriction {
        member_index: 2,
        restriction_type: 1,
        argument: ArgumentType::Bool(Box::new(true)),
        extensions: vec![],
    }
}

fn sample_custom_authority_create_operation() -> CustomAuthorityCreateOperation {
    CustomAuthorityCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        account: AccountId("1.2.1".to_string()),
        enabled: true,
        valid_from: "1970-01-01T00:00:01".to_string(),
        valid_to: "1970-01-01T00:00:02".to_string(),
        operation_type: 0,
        auth: sample_authority(),
        restrictions: vec![sample_restriction()],
        extensions: vec![],
    }
}

fn sample_custom_authority_update_operation() -> CustomAuthorityUpdateOperation {
    CustomAuthorityUpdateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        account: AccountId("1.2.1".to_string()),
        authority_to_update: CustomAuthorityId("1.17.1".to_string()),
        new_enabled: Some(false),
        new_valid_from: None,
        new_valid_to: Some("1970-01-01T00:00:03".to_string()),
        new_auth: None,
        restrictions_to_remove: vec![1, 3],
        restrictions_to_add: vec![sample_restriction()],
        extensions: vec![],
    }
}

fn sample_withdraw_permission_create_operation() -> WithdrawPermissionCreateOperation {
    WithdrawPermissionCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        withdraw_from_account: AccountId("1.2.1".to_string()),
        authorized_account: AccountId("1.2.2".to_string()),
        withdrawal_limit: Asset {
            amount: 3,
            asset_id: AssetId("1.3.0".to_string()),
        },
        withdrawal_period_sec: 86_400,
        periods_until_expiration: 7,
        period_start_time: "1970-01-01T00:00:01".to_string(),
    }
}

fn sample_account_create_operation() -> AccountCreateOperation {
    AccountCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        registrar: AccountId("1.2.1".to_string()),
        referrer: AccountId("1.2.2".to_string()),
        referrer_percent: 0,
        name: "alice".to_string(),
        owner: sample_authority(),
        active: sample_authority(),
        options: sample_account_options(),
        extensions: AccountCreateOperationExt {
            null_ext: None,
            owner_special_authority: None,
            active_special_authority: None,
            buyback_options: None,
        },
    }
}

fn sample_asset_update_feed_producers_operation() -> AssetUpdateFeedProducersOperation {
    AssetUpdateFeedProducersOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        issuer: AccountId("1.2.1".to_string()),
        asset_to_update: AssetId("1.3.4".to_string()),
        new_feed_producers: vec![
            AccountId("1.2.2".to_string()),
            AccountId("1.2.5".to_string()),
        ],
        extensions: vec![],
    }
}

fn sample_custom_operation() -> CustomOperation {
    CustomOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        payer: AccountId("1.2.1".to_string()),
        required_auths: vec![AccountId("1.2.1".to_string())],
        id: 0x1234,
        data: vec![0xab, 0xcd],
    }
}

fn sample_assert_operation() -> AssertOperation {
    AssertOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        fee_paying_account: AccountId("1.2.1".to_string()),
        predicates: vec![Predicate::BlockIdPredicate(Box::new(BlockIdPredicate {
            id: vec![0x44; 20],
        }))],
        required_auths: vec![AccountId("1.2.1".to_string())],
        extensions: vec![],
    }
}

fn sample_vesting_balance_create_operation() -> VestingBalanceCreateOperation {
    VestingBalanceCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        creator: AccountId("1.2.1".to_string()),
        owner: AccountId("1.2.2".to_string()),
        amount: Asset {
            amount: 5,
            asset_id: AssetId("1.3.0".to_string()),
        },
        policy: VestingPolicyInitializer::LinearVestingPolicyInitializer(Box::new(
            LinearVestingPolicyInitializer {
                begin_timestamp: "2020-01-01T00:00:00".to_string(),
                vesting_cliff_seconds: 10,
                vesting_duration_seconds: 20,
            },
        )),
    }
}

fn sample_worker_create_operation() -> WorkerCreateOperation {
    WorkerCreateOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        owner: AccountId("1.2.1".to_string()),
        work_begin_date: "2020-01-01T00:00:00".to_string(),
        work_end_date: "2020-01-02T00:00:00".to_string(),
        daily_pay: 1000,
        name: "worker".to_string(),
        url: "https://example.test".to_string(),
        initializer: WorkerInitializer::VestingBalanceWorkerInitializer(Box::new(
            VestingBalanceWorkerInitializer {
                pay_vesting_period_days: 7,
            },
        )),
    }
}

fn sample_htlc_refund_operation() -> HtlcRefundOperation {
    HtlcRefundOperation {
        fee: Asset {
            amount: 0,
            asset_id: AssetId("1.3.0".to_string()),
        },
        htlc_id: HtlcId("1.16.3".to_string()),
        to: AccountId("1.2.1".to_string()),
        original_htlc_recipient: AccountId("1.2.2".to_string()),
        htlc_amount: Asset {
            amount: 5,
            asset_id: AssetId("1.3.0".to_string()),
        },
        htlc_preimage_hash: HtlcHash::HtlcAlgoSha256(Box::new(vec![0x11; 32])),
        htlc_preimage_size: 32,
    }
}

fn sample_account_options() -> AccountOptions {
    AccountOptions {
        memo_key: "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV".to_string(),
        voting_account: AccountId("1.2.5".to_string()),
        num_witness: 1,
        num_committee: 2,
        votes: vec!["1:5".to_string()],
        extensions: vec![],
    }
}

fn sample_authority() -> Authority {
    Authority {
        weight_threshold: 1,
        account_auths: Vec::new(),
        key_auths: Vec::new(),
        address_auths: Vec::new(),
    }
}

fn sample_authority_with_account_and_key_auths() -> Authority {
    Authority {
        weight_threshold: 2,
        account_auths: vec![(AccountId("1.2.7".to_string()), 1)],
        key_auths: vec![(
            "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV".to_string(),
            2,
        )],
        address_auths: Vec::new(),
    }
}

fn generated_test_public_key(index: u8) -> String {
    match index {
        1 => "BTS4tVMTu4hrMTGeAQpAEzueCYqEESJQgkaH9DVJNnzK1mzPCmP45".to_string(),
        2 => "BTS4tVMTu4hrMTGeAQpAEzueCYqEESJQgkaH9DVJNnzK1mzWbbSw6".to_string(),
        _ => panic!("unsupported test public key index"),
    }
}

fn sample_authority_with_two_key_auths() -> Authority {
    Authority {
        weight_threshold: 2,
        account_auths: Vec::new(),
        key_auths: vec![
            (generated_test_public_key(1), 1),
            (generated_test_public_key(2), 2),
        ],
        address_auths: Vec::new(),
    }
}

fn expected_limit_order_create_payload() -> Vec<u8> {
    vec![
        // fee: amount 0 + asset instance 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, // seller account instance 1
        1, // amount_to_sell: amount 1 + asset instance 0
        1, 0, 0, 0, 0, 0, 0, 0, 0, // min_to_receive: amount 2 + asset instance 0
        2, 0, 0, 0, 0, 0, 0, 0, 0, 252, 57, 20, 106, // expiration 2026-05-25T12:01:00
        1,   // fill_or_kill true
        0,   // extensions: future_extensions VoidT static variant tag 0
    ]
}

fn expected_limit_order_cancel_payload() -> Vec<u8> {
    vec![
        // fee: amount 0 + asset instance 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, // fee_paying_account account instance 1
        1, // order limit_order instance 1
        1, // extensions: future_extensions VoidT static variant tag 0
        0,
    ]
}

fn expected_limit_order_update_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, seller account instance 1, order instance 1
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
    // new_price None, delta_amount_to_sell None, new_expiration None
    bytes.extend_from_slice(&[0, 0, 0]);
    // on_fill Some(vector len 1), create_take_profit_order_action tag 0
    bytes.extend_from_slice(&[1, 1, 0]);
    // action: fee_asset_id asset instance 0
    bytes.push(0);
    // spread_percent 25, size_percent 50, expiration_seconds 3600, repeat true, extensions tag 0
    bytes.extend_from_slice(&[25, 0, 50, 0, 16, 14, 0, 0, 1, 0]);
    // operation extensions tag 0
    bytes.push(0);
    bytes
}

fn expected_price_payload() -> Vec<u8> {
    vec![
        // base asset: amount 1 + asset instance 0
        1, 0, 0, 0, 0, 0, 0, 0, 0, // quote asset: amount 2 + asset instance 1
        2, 0, 0, 0, 0, 0, 0, 0, 1,
    ]
}

fn expected_credit_offer_create_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, owner account 1, asset_type 0
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0]);
    // balance 1000, fee_rate 25, max_duration 3600, min_deal_amount 10, enabled true
    bytes.extend_from_slice(&[232, 3, 0, 0, 0, 0, 0, 0]);
    bytes.extend_from_slice(&[25, 0, 0, 0]);
    bytes.extend_from_slice(&[16, 14, 0, 0]);
    bytes.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 1]);
    // auto_disable_time 2020-01-01T00:00:00
    bytes.extend_from_slice(&[0, 225, 11, 94]);
    // acceptable_collateral flat_map len 1, asset key 0, price payload
    bytes.extend_from_slice(&[1, 0]);
    bytes.extend(expected_price_payload());
    // acceptable_borrowers flat_map len 1, account key 1, i64 value 99
    bytes.extend_from_slice(&[1, 1, 99, 0, 0, 0, 0, 0, 0, 0]);
    // extensions tag 0
    bytes.push(0);
    bytes
}

fn expected_credit_offer_update_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, owner account 1, offer id instance 1
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
    // delta_amount, fee_rate, max_duration, min_deal_amount, enabled, auto_disable_time all None
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    // acceptable_collateral Some(flat_map len 1, asset key 0, price payload)
    bytes.extend_from_slice(&[1, 1, 0]);
    bytes.extend(expected_price_payload());
    // acceptable_borrowers Some(flat_map len 1, account key 1, i64 value 99)
    bytes.extend_from_slice(&[1, 1, 1, 99, 0, 0, 0, 0, 0, 0, 0]);
    // extensions tag 0
    bytes.push(0);
    bytes
}

fn expected_fee_schedule_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // parameters set length 2
    bytes.push(2);
    // transfer_operation_fee_params_t tag 0, fee u64 5, price_per_kbyte u32 6
    bytes.push(0);
    bytes.extend_from_slice(&[5, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend_from_slice(&[6, 0, 0, 0]);
    // committee_member_update_global_parameters_operation_fee_params_t tag 31, fee u64 7
    bytes.push(31);
    bytes.extend_from_slice(&[7, 0, 0, 0, 0, 0, 0, 0]);
    // scale u32 100
    bytes.extend_from_slice(&[100, 0, 0, 0]);
    bytes
}

fn expected_chain_parameters_payload() -> Vec<u8> {
    let mut bytes = expected_fee_schedule_payload();
    // remaining scalar chain_parameters fields are zero in sample_chain_parameters, then the empty
    // chain_parameters_ext extension set is a single varint 0.
    bytes.extend(vec![0; 85]);
    bytes
}

fn expected_committee_member_update_global_parameters_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend(expected_chain_parameters_payload());
    bytes
}

fn expected_restriction_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // member_index 2, restriction_type 1
    bytes.extend_from_slice(&[2, 0, 0, 0, 1, 0, 0, 0]);
    // argument_type Bool tag 1, true
    bytes.extend_from_slice(&[1, 1]);
    // extensions tag 0
    bytes.push(0);
    bytes
}

fn expected_custom_authority_create_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, account instance 1, enabled true
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
    // valid_from 1, valid_to 2, operation_type 0
    bytes.extend_from_slice(&[1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend(expected_empty_authority_payload());
    // restrictions vec length 1
    bytes.push(1);
    bytes.extend(expected_restriction_payload());
    // extensions tag 0
    bytes.push(0);
    bytes
}

fn expected_custom_authority_update_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, account instance 1, authority_to_update instance 1
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
    // new_enabled Some(false), new_valid_from None, new_valid_to Some(3), new_auth None
    bytes.extend_from_slice(&[1, 0, 0, 1, 3, 0, 0, 0, 0]);
    // restrictions_to_remove sorted set [1, 3]
    bytes.extend_from_slice(&[2, 1, 0, 3, 0]);
    // restrictions_to_add vec length 1
    bytes.push(1);
    bytes.extend(expected_restriction_payload());
    // extensions tag 0
    bytes.push(0);
    bytes
}

fn expected_withdraw_permission_create_payload() -> Vec<u8> {
    vec![
        // fee: amount 0 + asset instance 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, // withdraw_from_account account instance 1
        1, // authorized_account account instance 2
        2, // withdrawal_limit: amount 3 + asset instance 0
        3, 0, 0, 0, 0, 0, 0, 0, 0, // withdrawal_period_sec 86400
        0x80, 0x51, 0x01, 0x00, // periods_until_expiration 7
        7, 0, 0, 0, // period_start_time 1970-01-01T00:00:01 as u32 little-endian seconds
        1, 0, 0, 0,
    ]
}

fn expected_account_options_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // memo_key: compressed public key bytes
    bytes.extend_from_slice(&[
        0x02, 0xc0, 0xde, 0xd2, 0xbc, 0x1f, 0x13, 0x05, 0xfb, 0x0f, 0xaa, 0xc5, 0xe6, 0xc0, 0x3e,
        0xe3, 0xa1, 0x92, 0x42, 0x34, 0x98, 0x54, 0x27, 0xb6, 0x16, 0x7c, 0xa5, 0x69, 0xd1, 0x3d,
        0xf4, 0x35, 0xcf,
    ]);
    // voting_account account instance 5
    bytes.push(5);
    // num_witness 1, num_committee 2
    bytes.extend_from_slice(&[1, 0, 2, 0]);
    // votes set length 1, vote_id "1:5" => content (5 << 8) | 1
    bytes.extend_from_slice(&[1, 1, 5, 0, 0]);
    // extensions: future_extensions VoidT static variant tag 0
    bytes.push(0);
    bytes
}

fn expected_authority_with_account_and_key_auths_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // weight_threshold 2
    bytes.extend_from_slice(&[2, 0, 0, 0]);
    // account_auths length 1, account instance 7, weight 1
    bytes.extend_from_slice(&[1, 7, 1, 0]);
    // key_auths length 1, public key bytes, weight 2
    bytes.push(1);
    bytes.extend_from_slice(&[
        0x02, 0xc0, 0xde, 0xd2, 0xbc, 0x1f, 0x13, 0x05, 0xfb, 0x0f, 0xaa, 0xc5, 0xe6, 0xc0, 0x3e,
        0xe3, 0xa1, 0x92, 0x42, 0x34, 0x98, 0x54, 0x27, 0xb6, 0x16, 0x7c, 0xa5, 0x69, 0xd1, 0x3d,
        0xf4, 0x35, 0xcf,
    ]);
    bytes.extend_from_slice(&[2, 0]);
    // address_auths length 0
    bytes.push(0);
    bytes
}

fn expected_authority_with_two_key_auths_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // weight_threshold 2, account_auths length 0, key_auths length 2
    bytes.extend_from_slice(&[2, 0, 0, 0, 0, 2]);
    // compressed public key 0x02 || 1, weight 1
    bytes.push(0x02);
    bytes.extend_from_slice(&[0; 31]);
    bytes.push(1);
    bytes.extend_from_slice(&[1, 0]);
    // compressed public key 0x02 || 2, weight 2
    bytes.push(0x02);
    bytes.extend_from_slice(&[0; 31]);
    bytes.push(2);
    bytes.extend_from_slice(&[2, 0]);
    // address_auths length 0
    bytes.push(0);
    bytes
}

fn expected_empty_authority_payload() -> Vec<u8> {
    vec![
        // weight_threshold 1
        1, 0, 0, 0, // account_auths, key_auths, address_auths lengths
        0, 0, 0,
    ]
}

fn expected_account_create_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // registrar, referrer, referrer_percent
    bytes.extend_from_slice(&[1, 2, 0, 0]);
    // name "alice"
    bytes.extend_from_slice(&[5, b'a', b'l', b'i', b'c', b'e']);
    bytes.extend(expected_empty_authority_payload());
    bytes.extend(expected_empty_authority_payload());
    bytes.extend(expected_account_options_payload());
    // account_create_operation_ext: empty Graphene extension set
    bytes.push(0);
    bytes
}

fn expected_asset_update_feed_producers_payload() -> Vec<u8> {
    vec![
        // fee: amount 0 + asset instance 0
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        // issuer account instance 1, asset_to_update asset instance 4
        1, 4, // new_feed_producers set length 2, account instances 2 and 5
        2, 2, 5, // extensions: future_extensions VoidT static variant tag 0
        0,
    ]
}

fn expected_custom_operation_payload() -> Vec<u8> {
    vec![
        // fee: amount 0 + asset instance 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, // payer account instance 1
        1, // required_auths set length 1, account instance 1
        1, 1, // id 0x1234 little-endian
        0x34, 0x12, // data bytes length 2 + raw bytes
        2, 0xab, 0xcd,
    ]
}

fn expected_assert_operation_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, fee_paying_account instance 1
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    // predicates length 1, block_id_predicate tag 2 + 20 fixed bytes, no length prefix
    bytes.extend_from_slice(&[1, 2]);
    bytes.extend_from_slice(&[0x44; 20]);
    // required_auths set length 1, account instance 1, extensions future_extensions tag 0
    bytes.extend_from_slice(&[1, 1, 0]);
    bytes
}

fn expected_vesting_balance_create_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, creator 1, owner 2
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2]);
    // amount: amount 5 + asset instance 0
    bytes.extend_from_slice(&[5, 0, 0, 0, 0, 0, 0, 0, 0]);
    // linear_vesting_policy_initializer tag 0
    bytes.push(0);
    // begin_timestamp 2020-01-01T00:00:00 + cliff 10 + duration 20
    bytes.extend_from_slice(&[0, 225, 11, 94]);
    bytes.extend_from_slice(&[10, 0, 0, 0]);
    bytes.extend_from_slice(&[20, 0, 0, 0]);
    bytes
}

fn expected_worker_create_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, owner account instance 1
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    // work_begin_date 2020-01-01T00:00:00, work_end_date 2020-01-02T00:00:00
    bytes.extend_from_slice(&[0, 225, 11, 94]);
    bytes.extend_from_slice(&[128, 50, 13, 94]);
    // daily_pay 1000 i64 little-endian
    bytes.extend_from_slice(&[232, 3, 0, 0, 0, 0, 0, 0]);
    // name and URL as length-prefixed UTF-8 strings
    bytes.extend_from_slice(&[6, b'w', b'o', b'r', b'k', b'e', b'r']);
    bytes.extend_from_slice(&[
        20, b'h', b't', b't', b'p', b's', b':', b'/', b'/', b'e', b'x', b'a', b'm', b'p', b'l',
        b'e', b'.', b't', b'e', b's', b't',
    ]);
    // vesting_balance_worker_initializer tag 1 + pay_vesting_period_days 7
    bytes.extend_from_slice(&[1, 7, 0]);
    bytes
}

fn expected_htlc_refund_payload() -> Vec<u8> {
    let mut bytes = Vec::new();
    // fee amount 0 + asset instance 0, htlc id instance 3, to account 1, original recipient 2
    bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 1, 2]);
    // htlc_amount: amount 5 + asset instance 0
    bytes.extend_from_slice(&[5, 0, 0, 0, 0, 0, 0, 0, 0]);
    // htlc_hash static variant tag 2 + 32 fixed bytes, no length prefix
    bytes.push(2);
    bytes.extend_from_slice(&[0x11; 32]);
    // htlc_preimage_size 32
    bytes.extend_from_slice(&[32, 0]);
    bytes
}

#[test]
fn asset_fc_serializes_amount_and_asset_instance() {
    let asset = Asset {
        amount: 100_000,
        asset_id: AssetId("1.3.0".to_string()),
    };

    assert_eq!(
        asset.to_fc_bytes().expect("serialize asset"),
        vec![0xa0, 0x86, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn transfer_operation_fc_rejects_invalid_account_id() {
    let mut transfer = sample_transfer_operation();
    transfer.from = AccountId("bad".to_string());

    let err = transfer
        .to_fc_bytes()
        .expect_err("invalid account id fails");

    assert!(matches!(
        err,
        FcSerializeError::InvalidProtocolObjectId {
            expected_space: Some(1),
            expected_type: Some(2),
            ..
        }
    ));
}

#[test]
fn account_options_fc_serializes_vote_ids() {
    assert_eq!(
        sample_account_options()
            .to_fc_bytes()
            .expect("serialize account options"),
        expected_account_options_payload()
    );
}

#[test]
fn account_options_fc_rejects_invalid_vote_id() {
    let mut options = sample_account_options();
    options.votes = vec!["1:16777216".to_string()];

    let err = options.to_fc_bytes().expect_err("invalid vote id fails");

    assert!(matches!(err, FcSerializeError::InvalidVoteId { .. }));
}

#[test]
fn authority_fc_serializes_account_and_key_auths_with_empty_address_auths() {
    assert_eq!(
        sample_authority_with_account_and_key_auths()
            .to_fc_bytes()
            .expect("serialize authority"),
        expected_authority_with_account_and_key_auths_payload()
    );
}

#[test]
fn authority_fc_orders_public_key_flat_map_by_compressed_key_bytes() {
    assert_eq!(
        sample_authority_with_two_key_auths()
            .to_fc_bytes()
            .expect("serialize authority with two key_auths"),
        expected_authority_with_two_key_auths_payload()
    );

    let mut authority = sample_authority_with_two_key_auths();
    authority.key_auths.reverse();

    let err = authority
        .to_fc_bytes()
        .expect_err("public key flat_map ordering is byte-canonical");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "FlatMap",
            reason: "flat_map keys must be sorted and unique"
        }
    ));
}

#[test]
fn authority_fc_rejects_non_empty_address_auths() {
    let mut authority = sample_authority();
    authority.address_auths = vec![("BTS1111111111111111111111111111111114T1Anm".to_string(), 1)];

    let err = authority
        .to_fc_bytes()
        .expect_err("non-empty address_auths is explicitly unsupported");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Address",
            reason: "address flat_map FC serialization is not implemented"
        }
    ));
}

#[test]
fn authority_fc_rejects_unsorted_or_duplicate_flat_map_keys() {
    let mut authority = sample_authority();
    authority.account_auths = vec![
        (AccountId("1.2.8".to_string()), 1),
        (AccountId("1.2.7".to_string()), 1),
    ];

    let err = authority
        .to_fc_bytes()
        .expect_err("unsorted flat_map keys fail explicitly");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "FlatMap",
            reason: "flat_map keys must be sorted and unique"
        }
    ));

    authority.account_auths = vec![
        (AccountId("1.2.7".to_string()), 1),
        (AccountId("1.2.7".to_string()), 2),
    ];

    let err = authority
        .to_fc_bytes()
        .expect_err("duplicate flat_map keys fail explicitly");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "FlatMap",
            reason: "flat_map keys must be sorted and unique"
        }
    ));
}

#[test]
fn operation_fc_serializes_transfer_tag_and_payload() {
    let operation = Operation::TransferOperation(Box::new(sample_transfer_operation()));
    let bytes = operation.to_fc_bytes().expect("serialize operation");

    assert_eq!(bytes[0], 0);
    assert_eq!(bytes[1..9], [0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(bytes[9], 0); // fee asset instance 0
    assert_eq!(bytes[10], 1); // from account instance 1
    assert_eq!(bytes[11], 2); // to account instance 2
}

#[test]
fn limit_order_create_operation_fc_serializes_known_fields() {
    let operation = sample_limit_order_create_operation();

    assert_eq!(
        operation
            .to_fc_bytes()
            .expect("serialize limit order create"),
        expected_limit_order_create_payload()
    );
}

#[test]
fn operation_fc_serializes_limit_order_create_tag_and_payload() {
    let operation =
        Operation::LimitOrderCreateOperation(Box::new(sample_limit_order_create_operation()));
    let bytes = operation.to_fc_bytes().expect("serialize operation");

    let mut expected = vec![1];
    expected.extend(expected_limit_order_create_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn limit_order_cancel_operation_fc_serializes_known_fields() {
    let operation = sample_limit_order_cancel_operation();

    assert_eq!(
        operation
            .to_fc_bytes()
            .expect("serialize limit order cancel"),
        expected_limit_order_cancel_payload()
    );
}

#[test]
fn operation_fc_serializes_limit_order_cancel_tag_and_payload() {
    let operation =
        Operation::LimitOrderCancelOperation(Box::new(sample_limit_order_cancel_operation()));
    let bytes = operation.to_fc_bytes().expect("serialize operation");

    let mut expected = vec![2];
    expected.extend(expected_limit_order_cancel_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn limit_order_auto_action_fc_serializes_take_profit_action() {
    assert_eq!(
        LimitOrderAutoAction::CreateTakeProfitOrderAction(Box::new(CreateTakeProfitOrderAction {
            fee_asset_id: AssetId("1.3.0".to_string()),
            spread_percent: 25,
            size_percent: 50,
            expiration_seconds: 3_600,
            repeat: true,
            extensions: vec![],
        }))
        .to_fc_bytes()
        .expect("serialize take profit auto action"),
        vec![0, 0, 25, 0, 50, 0, 16, 14, 0, 0, 1, 0]
    );
}

#[test]
fn limit_order_update_operation_fc_serializes_on_fill_action() {
    assert_eq!(
        sample_limit_order_update_operation()
            .to_fc_bytes()
            .expect("serialize limit order update"),
        expected_limit_order_update_payload()
    );
}

#[test]
fn operation_fc_serializes_limit_order_update_tag_and_payload() {
    let operation =
        Operation::LimitOrderUpdateOperation(Box::new(sample_limit_order_update_operation()));
    let bytes = operation
        .to_fc_bytes()
        .expect("serialize limit order update variant");

    let mut expected = vec![77];
    expected.extend(expected_limit_order_update_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn credit_offer_create_operation_fc_serializes_flat_maps() {
    assert_eq!(
        sample_credit_offer_create_operation()
            .to_fc_bytes()
            .expect("serialize credit offer create"),
        expected_credit_offer_create_payload()
    );
}

#[test]
fn credit_offer_update_operation_fc_serializes_optional_flat_maps() {
    assert_eq!(
        sample_credit_offer_update_operation()
            .to_fc_bytes()
            .expect("serialize credit offer update"),
        expected_credit_offer_update_payload()
    );
}

#[test]
fn credit_offer_flat_maps_reject_unsorted_or_duplicate_keys() {
    let mut operation = sample_credit_offer_create_operation();
    operation.acceptable_collateral = vec![
        (AssetId("1.3.2".to_string()), sample_price()),
        (AssetId("1.3.1".to_string()), sample_price()),
    ];

    let err = operation
        .to_fc_bytes()
        .expect_err("unsorted asset flat_map keys fail explicitly");
    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "FlatMap",
            reason: "flat_map keys must be sorted and unique"
        }
    ));

    operation.acceptable_collateral = vec![
        (AssetId("1.3.1".to_string()), sample_price()),
        (AssetId("1.3.1".to_string()), sample_price()),
    ];

    let err = operation
        .to_fc_bytes()
        .expect_err("duplicate asset flat_map keys fail explicitly");
    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "FlatMap",
            reason: "flat_map keys must be sorted and unique"
        }
    ));
}

#[test]
fn operation_fc_serializes_credit_offer_tags_and_payloads() {
    let bytes =
        Operation::CreditOfferCreateOperation(Box::new(sample_credit_offer_create_operation()))
            .to_fc_bytes()
            .expect("serialize credit offer create variant");
    let mut expected = vec![69];
    expected.extend(expected_credit_offer_create_payload());
    assert_eq!(bytes, expected);

    let bytes =
        Operation::CreditOfferUpdateOperation(Box::new(sample_credit_offer_update_operation()))
            .to_fc_bytes()
            .expect("serialize credit offer update variant");
    let mut expected = vec![71];
    expected.extend(expected_credit_offer_update_payload());
    assert_eq!(bytes, expected);
}

#[test]
fn fee_parameters_fc_serializes_known_variants() {
    assert_eq!(
        FeeParameters::TransferOperationFeeParamsT(Box::new(TransferOperationFeeParamsT {
            fee: 5,
            price_per_kbyte: 6,
        }))
        .to_fc_bytes()
        .expect("serialize transfer fee params"),
        vec![0, 5, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0]
    );

    assert_eq!(
        FeeParameters::CommitteeMemberUpdateGlobalParametersOperationFeeParamsT(Box::new(
            CommitteeMemberUpdateGlobalParametersOperationFeeParamsT { fee: 7 },
        ))
        .to_fc_bytes()
        .expect("serialize committee global params fee params"),
        vec![31, 7, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn fee_schedule_fc_serializes_sorted_fee_parameters_set() {
    assert_eq!(
        sample_fee_schedule()
            .to_fc_bytes()
            .expect("serialize fee schedule"),
        expected_fee_schedule_payload()
    );
}

#[test]
fn fee_schedule_fc_rejects_unsorted_or_duplicate_fee_parameter_tags() {
    let mut schedule = sample_fee_schedule();
    schedule.parameters.reverse();

    let err = schedule
        .to_fc_bytes()
        .expect_err("unsorted fee parameter set fails explicitly");
    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Set",
            reason: "set values must be sorted and unique"
        }
    ));

    schedule.parameters = vec![
        FeeParameters::TransferOperationFeeParamsT(Box::new(TransferOperationFeeParamsT {
            fee: 5,
            price_per_kbyte: 6,
        })),
        FeeParameters::TransferOperationFeeParamsT(Box::new(TransferOperationFeeParamsT {
            fee: 8,
            price_per_kbyte: 9,
        })),
    ];

    let err = schedule
        .to_fc_bytes()
        .expect_err("duplicate fee parameter set tags fail explicitly");
    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Set",
            reason: "set values must be sorted and unique"
        }
    ));
}

#[test]
fn committee_member_update_global_parameters_operation_fc_serializes_fee_schedule() {
    assert_eq!(
        sample_committee_member_update_global_parameters_operation()
            .to_fc_bytes()
            .expect("serialize committee global parameters update"),
        expected_committee_member_update_global_parameters_payload()
    );
}

#[test]
fn operation_fc_serializes_committee_member_update_global_parameters_tag_and_payload() {
    let bytes = Operation::CommitteeMemberUpdateGlobalParametersOperation(Box::new(
        sample_committee_member_update_global_parameters_operation(),
    ))
    .to_fc_bytes()
    .expect("serialize committee global parameters update variant");

    let mut expected = vec![31];
    expected.extend(expected_committee_member_update_global_parameters_payload());
    assert_eq!(bytes, expected);
}

#[test]
fn argument_type_fc_serializes_scalar_set_and_recursive_pair_variants() {
    assert_eq!(
        ArgumentType::TimePointSec(Box::new("1970-01-01T00:00:03".to_string()))
            .to_fc_bytes()
            .expect("serialize time_point_sec argument"),
        vec![4, 3, 0, 0, 0]
    );

    assert_eq!(
        ArgumentType::FlatSetInt64T(Box::new(vec![-1, 2]))
            .to_fc_bytes()
            .expect("serialize sorted int64 set argument"),
        vec![
            21, 2, 255, 255, 255, 255, 255, 255, 255, 255, 2, 0, 0, 0, 0, 0, 0, 0
        ]
    );

    let bytes = ArgumentType::VariantAssertArgumentType(Box::new((9, vec![sample_restriction()])))
        .to_fc_bytes()
        .expect("serialize recursive variant assert argument");
    let mut expected = vec![41, 9, 0, 0, 0, 0, 0, 0, 0, 1];
    expected.extend(expected_restriction_payload());
    assert_eq!(bytes, expected);
}

#[test]
fn argument_type_fc_rejects_unsorted_or_duplicate_sets() {
    let err = ArgumentType::FlatSetInt64T(Box::new(vec![2, -1]))
        .to_fc_bytes()
        .expect_err("unsorted argument set fails explicitly");
    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Set",
            reason: "set values must be sorted and unique"
        }
    ));

    let err = ArgumentType::FlatSetString(Box::new(vec!["same".to_string(), "same".to_string()]))
        .to_fc_bytes()
        .expect_err("duplicate argument set fails explicitly");
    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Set",
            reason: "set values must be sorted and unique"
        }
    ));
}

#[test]
fn custom_authority_operations_fc_serialize_argument_restrictions() {
    assert_eq!(
        sample_custom_authority_create_operation()
            .to_fc_bytes()
            .expect("serialize custom authority create"),
        expected_custom_authority_create_payload()
    );

    assert_eq!(
        sample_custom_authority_update_operation()
            .to_fc_bytes()
            .expect("serialize custom authority update"),
        expected_custom_authority_update_payload()
    );
}

#[test]
fn custom_authority_update_fc_rejects_unsorted_or_duplicate_removed_restrictions() {
    let mut op = sample_custom_authority_update_operation();
    op.restrictions_to_remove = vec![3, 1];
    let err = op
        .to_fc_bytes()
        .expect_err("unsorted restriction removal set fails explicitly");
    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Set",
            reason: "set values must be sorted and unique"
        }
    ));

    op.restrictions_to_remove = vec![1, 1];
    let err = op
        .to_fc_bytes()
        .expect_err("duplicate restriction removal set fails explicitly");
    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Set",
            reason: "set values must be sorted and unique"
        }
    ));
}

#[test]
fn operation_fc_serializes_custom_authority_tags_and_payloads() {
    let bytes = Operation::CustomAuthorityCreateOperation(Box::new(
        sample_custom_authority_create_operation(),
    ))
    .to_fc_bytes()
    .expect("serialize custom authority create variant");
    let mut expected = vec![54];
    expected.extend(expected_custom_authority_create_payload());
    assert_eq!(bytes, expected);

    let bytes = Operation::CustomAuthorityUpdateOperation(Box::new(
        sample_custom_authority_update_operation(),
    ))
    .to_fc_bytes()
    .expect("serialize custom authority update variant");
    let mut expected = vec![55];
    expected.extend(expected_custom_authority_update_payload());
    assert_eq!(bytes, expected);
}

#[test]
fn withdraw_permission_create_operation_fc_serializes_time_point_sec() {
    let operation = sample_withdraw_permission_create_operation();

    assert_eq!(
        operation
            .to_fc_bytes()
            .expect("serialize withdraw permission create"),
        expected_withdraw_permission_create_payload()
    );
}

#[test]
fn withdraw_permission_create_operation_rejects_invalid_time_point_sec() {
    let mut operation = sample_withdraw_permission_create_operation();
    operation.period_start_time = "1970-01-01T00:00:00Z".to_string();

    let err = operation
        .to_fc_bytes()
        .expect_err("ambiguous timestamp format fails");

    assert!(matches!(err, FcSerializeError::InvalidTimePointSec { .. }));
}

#[test]
fn operation_fc_serializes_withdraw_permission_create_tag_and_payload() {
    let operation = Operation::WithdrawPermissionCreateOperation(Box::new(
        sample_withdraw_permission_create_operation(),
    ));
    let bytes = operation.to_fc_bytes().expect("serialize operation");

    let mut expected = vec![25];
    expected.extend(expected_withdraw_permission_create_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn special_authority_fc_serializes_known_variants() {
    assert_eq!(
        SpecialAuthority::NoSpecialAuthority(Box::new(NoSpecialAuthority {}))
            .to_fc_bytes()
            .expect("serialize no special authority"),
        vec![0]
    );

    assert_eq!(
        SpecialAuthority::TopHoldersSpecialAuthority(Box::new(TopHoldersSpecialAuthority {
            asset: AssetId("1.3.5".to_string()),
            num_top_holders: 7,
        }))
        .to_fc_bytes()
        .expect("serialize top holders special authority"),
        vec![1, 5, 7]
    );
}

#[test]
fn account_create_operation_fc_serializes_empty_special_authorities() {
    assert_eq!(
        sample_account_create_operation()
            .to_fc_bytes()
            .expect("serialize account create"),
        expected_account_create_payload()
    );
}

#[test]
fn operation_fc_serializes_account_create_tag_and_payload() {
    let bytes = Operation::AccountCreateOperation(Box::new(sample_account_create_operation()))
        .to_fc_bytes()
        .expect("serialize account create operation");

    let mut expected = vec![5];
    expected.extend(expected_account_create_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn asset_update_feed_producers_operation_fc_serializes_sorted_account_set() {
    assert_eq!(
        sample_asset_update_feed_producers_operation()
            .to_fc_bytes()
            .expect("serialize asset update feed producers"),
        expected_asset_update_feed_producers_payload()
    );
}

#[test]
fn operation_fc_serializes_asset_update_feed_producers_tag_and_payload() {
    let bytes = Operation::AssetUpdateFeedProducersOperation(Box::new(
        sample_asset_update_feed_producers_operation(),
    ))
    .to_fc_bytes()
    .expect("serialize asset update feed producers operation");

    let mut expected = vec![13];
    expected.extend(expected_asset_update_feed_producers_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn custom_operation_fc_serializes_byte_payload() {
    assert_eq!(
        sample_custom_operation()
            .to_fc_bytes()
            .expect("serialize custom operation"),
        expected_custom_operation_payload()
    );
}

#[test]
fn operation_fc_serializes_custom_operation_tag_and_payload() {
    let bytes = Operation::CustomOperation(Box::new(sample_custom_operation()))
        .to_fc_bytes()
        .expect("serialize custom operation variant");

    let mut expected = vec![35];
    expected.extend(expected_custom_operation_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn htlc_hash_fc_serializes_fixed_bytes_without_length_prefix() {
    assert_eq!(
        HtlcHash::HtlcAlgoRipemd160(Box::new(vec![0x22; 20]))
            .to_fc_bytes()
            .expect("serialize htlc ripemd160 hash"),
        [vec![0], vec![0x22; 20]].concat()
    );

    assert_eq!(
        HtlcHash::HtlcAlgoSha256(Box::new(vec![0x11; 32]))
            .to_fc_bytes()
            .expect("serialize htlc sha256 hash"),
        [vec![2], vec![0x11; 32]].concat()
    );
}

#[test]
fn htlc_hash_fc_rejects_invalid_fixed_length() {
    let err = HtlcHash::HtlcAlgoSha256(Box::new(vec![0x11; 31]))
        .to_fc_bytes()
        .expect_err("invalid fixed hash length fails");

    assert!(matches!(
        err,
        FcSerializeError::InvalidFixedBytes {
            type_name: "htlc_hash::htlc_algo_sha256",
            expected_len: 32,
            actual_len: 31,
        }
    ));
}

#[test]
fn predicate_fc_serializes_block_id_predicate() {
    assert_eq!(
        Predicate::BlockIdPredicate(Box::new(BlockIdPredicate { id: vec![0x44; 20] }))
            .to_fc_bytes()
            .expect("serialize block id predicate"),
        [vec![2], vec![0x44; 20]].concat()
    );
}

#[test]
fn predicate_fc_serializes_literal_predicates() {
    assert_eq!(
        Predicate::AccountNameEqLitPredicate(Box::new(AccountNameEqLitPredicate {
            account_id: AccountId("1.2.7".to_string()),
            name: "alice".to_string(),
        }))
        .to_fc_bytes()
        .expect("serialize account name predicate"),
        vec![0, 7, 5, b'a', b'l', b'i', b'c', b'e']
    );

    assert_eq!(
        Predicate::AssetSymbolEqLitPredicate(Box::new(AssetSymbolEqLitPredicate {
            asset_id: AssetId("1.3.4".to_string()),
            symbol: "TEST".to_string(),
        }))
        .to_fc_bytes()
        .expect("serialize asset symbol predicate"),
        vec![1, 4, 4, b'T', b'E', b'S', b'T']
    );
}

#[test]
fn predicate_fc_rejects_invalid_block_id_length() {
    let err = Predicate::BlockIdPredicate(Box::new(BlockIdPredicate { id: vec![0x44; 19] }))
        .to_fc_bytes()
        .expect_err("invalid block id length fails");

    assert!(matches!(
        err,
        FcSerializeError::InvalidFixedBytes {
            type_name: "fixed_bytes_20",
            expected_len: 20,
            actual_len: 19,
        }
    ));
}

#[test]
fn htlc_refund_operation_fc_serializes_preimage_hash() {
    assert_eq!(
        sample_htlc_refund_operation()
            .to_fc_bytes()
            .expect("serialize htlc refund"),
        expected_htlc_refund_payload()
    );
}

#[test]
fn operation_fc_serializes_htlc_refund_tag_and_payload() {
    let bytes = Operation::HtlcRefundOperation(Box::new(sample_htlc_refund_operation()))
        .to_fc_bytes()
        .expect("serialize htlc refund variant");

    let mut expected = vec![53];
    expected.extend(expected_htlc_refund_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn assert_operation_fc_serializes_predicates() {
    assert_eq!(
        sample_assert_operation()
            .to_fc_bytes()
            .expect("serialize assert operation"),
        expected_assert_operation_payload()
    );
}

#[test]
fn operation_fc_serializes_assert_tag_and_payload() {
    let bytes = Operation::AssertOperation(Box::new(sample_assert_operation()))
        .to_fc_bytes()
        .expect("serialize assert operation variant");

    let mut expected = vec![36];
    expected.extend(expected_assert_operation_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn vesting_policy_initializer_fc_serializes_known_variants() {
    assert_eq!(
        VestingPolicyInitializer::LinearVestingPolicyInitializer(Box::new(
            LinearVestingPolicyInitializer {
                begin_timestamp: "2020-01-01T00:00:00".to_string(),
                vesting_cliff_seconds: 10,
                vesting_duration_seconds: 20,
            },
        ))
        .to_fc_bytes()
        .expect("serialize linear vesting policy initializer"),
        vec![0, 0, 225, 11, 94, 10, 0, 0, 0, 20, 0, 0, 0]
    );

    assert_eq!(
        VestingPolicyInitializer::CddVestingPolicyInitializer(Box::new(
            CddVestingPolicyInitializer {
                start_claim: "2020-01-02T00:00:00".to_string(),
                vesting_seconds: 30,
            },
        ))
        .to_fc_bytes()
        .expect("serialize cdd vesting policy initializer"),
        vec![1, 128, 50, 13, 94, 30, 0, 0, 0]
    );

    assert_eq!(
        VestingPolicyInitializer::InstantVestingPolicyInitializer(Box::new(
            InstantVestingPolicyInitializer {},
        ))
        .to_fc_bytes()
        .expect("serialize instant vesting policy initializer"),
        vec![2]
    );
}

#[test]
fn vesting_policy_initializer_fc_rejects_invalid_time_point_sec() {
    let err = VestingPolicyInitializer::LinearVestingPolicyInitializer(Box::new(
        LinearVestingPolicyInitializer {
            begin_timestamp: "2020-01-01T00:00:00Z".to_string(),
            vesting_cliff_seconds: 10,
            vesting_duration_seconds: 20,
        },
    ))
    .to_fc_bytes()
    .expect_err("invalid timestamp fails");

    assert!(matches!(err, FcSerializeError::InvalidTimePointSec { .. }));
}

#[test]
fn vesting_balance_create_operation_fc_serializes_policy() {
    assert_eq!(
        sample_vesting_balance_create_operation()
            .to_fc_bytes()
            .expect("serialize vesting balance create"),
        expected_vesting_balance_create_payload()
    );
}

#[test]
fn operation_fc_serializes_vesting_balance_create_tag_and_payload() {
    let bytes = Operation::VestingBalanceCreateOperation(Box::new(
        sample_vesting_balance_create_operation(),
    ))
    .to_fc_bytes()
    .expect("serialize vesting balance create variant");

    let mut expected = vec![32];
    expected.extend(expected_vesting_balance_create_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn worker_initializer_fc_serializes_known_variants() {
    assert_eq!(
        WorkerInitializer::RefundWorkerInitializer(Box::new(RefundWorkerInitializer {}))
            .to_fc_bytes()
            .expect("serialize refund worker initializer"),
        vec![0]
    );

    assert_eq!(
        WorkerInitializer::VestingBalanceWorkerInitializer(Box::new(
            VestingBalanceWorkerInitializer {
                pay_vesting_period_days: 7,
            },
        ))
        .to_fc_bytes()
        .expect("serialize vesting balance worker initializer"),
        vec![1, 7, 0]
    );

    assert_eq!(
        WorkerInitializer::BurnWorkerInitializer(Box::new(BurnWorkerInitializer {}))
            .to_fc_bytes()
            .expect("serialize burn worker initializer"),
        vec![2]
    );
}

#[test]
fn worker_create_operation_fc_serializes_initializer() {
    assert_eq!(
        sample_worker_create_operation()
            .to_fc_bytes()
            .expect("serialize worker create"),
        expected_worker_create_payload()
    );
}

#[test]
fn worker_create_operation_fc_rejects_invalid_time_point_sec() {
    let mut operation = sample_worker_create_operation();
    operation.work_begin_date = "2020-01-01T00:00:00Z".to_string();

    let err = operation
        .to_fc_bytes()
        .expect_err("invalid worker timestamp fails");

    assert!(matches!(err, FcSerializeError::InvalidTimePointSec { .. }));
}

#[test]
fn operation_fc_serializes_worker_create_tag_and_payload() {
    let bytes = Operation::WorkerCreateOperation(Box::new(sample_worker_create_operation()))
        .to_fc_bytes()
        .expect("serialize worker create variant");

    let mut expected = vec![34];
    expected.extend(expected_worker_create_payload());

    assert_eq!(bytes, expected);
}

#[test]
fn account_id_set_fc_rejects_unsorted_or_duplicate_values() {
    let mut operation = sample_asset_update_feed_producers_operation();
    operation.new_feed_producers = vec![
        AccountId("1.2.5".to_string()),
        AccountId("1.2.2".to_string()),
    ];

    let err = operation
        .to_fc_bytes()
        .expect_err("unsorted set values fail explicitly");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Set",
            reason: "set values must be sorted and unique"
        }
    ));

    operation.new_feed_producers = vec![
        AccountId("1.2.2".to_string()),
        AccountId("1.2.2".to_string()),
    ];

    let err = operation
        .to_fc_bytes()
        .expect_err("duplicate set values fail explicitly");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "Set",
            reason: "set values must be sorted and unique"
        }
    ));
}

#[test]
fn transaction_fc_serializes_transfer_operation_vector() {
    let transaction = sample_transaction_with_operations(vec![Operation::TransferOperation(
        Box::new(sample_transfer_operation()),
    )]);

    let mut operation_payload = vec![0];
    operation_payload.extend(expected_transfer_payload());
    assert_eq!(
        transaction
            .to_fc_bytes()
            .expect("serialize transaction with transfer"),
        expected_transaction_payload(&[operation_payload])
    );
}

#[test]
fn transaction_fc_serializes_proposal_operation_vector() {
    let transaction = sample_transaction_with_operations(vec![Operation::ProposalCreateOperation(
        Box::new(sample_proposal_create_operation()),
    )]);

    let mut operation_payload = vec![22];
    operation_payload.extend(expected_proposal_create_payload());
    assert_eq!(
        transaction
            .to_fc_bytes()
            .expect("serialize transaction with proposal"),
        expected_transaction_payload(&[operation_payload])
    );
}

#[test]
fn signature_fc_serializes_exact_65_raw_bytes() {
    let signature = sample_signature_bytes();
    assert_eq!(
        Signature(signature.clone())
            .to_fc_bytes()
            .expect("serialize compact signature bytes"),
        signature
    );
}

#[test]
fn signature_fc_rejects_invalid_lengths() {
    for length in [64usize, 66] {
        let err = Signature(vec![0; length])
            .to_fc_bytes()
            .expect_err("invalid compact signature length fails");
        assert!(matches!(
            err,
            FcSerializeError::InvalidFixedBytes {
                type_name: "signature",
                expected_len: 65,
                actual_len,
            } if actual_len == length
        ));
    }
}

#[test]
fn signed_transaction_fc_serializes_transaction_fields_and_signature_vector() {
    let signature = sample_signature_bytes();
    let signed_transaction = sample_signed_transaction_with_operations(
        vec![Operation::TransferOperation(Box::new(
            sample_transfer_operation(),
        ))],
        vec![Signature(signature.clone())],
    );

    let mut operation_payload = vec![0];
    operation_payload.extend(expected_transfer_payload());
    assert_eq!(
        signed_transaction
            .to_fc_bytes()
            .expect("serialize signed transaction with transfer"),
        expected_signed_transaction_payload(&[operation_payload], &[signature])
    );
}

#[test]
fn signed_transaction_fc_matches_bitsharesjs_signature_fixture_bytes() {
    let signature = bitsharesjs_signature_vector_bytes();
    let signed_transaction = sample_signed_transaction_with_operations(
        vec![Operation::TransferOperation(Box::new(
            sample_transfer_operation(),
        ))],
        vec![Signature(signature)],
    );

    assert_eq!(
        signed_transaction
            .to_fc_bytes()
            .expect("serialize fixture signed transaction"),
        decode_hex(
            "0100020000000300000001000000000000000000000102a08601000000000000000000011f4a8b0c4b54ffd78ee503c2e9112932b2caa726086aa1acb72cc550e1fea0407e4765ad8766314c4e8eb2b42017ac1048ebece448639f63d8082a1e893009a874",
        )
    );
}

#[test]
fn signed_transaction_fc_propagates_signature_length_errors() {
    let signed_transaction = sample_signed_transaction_with_operations(
        vec![Operation::TransferOperation(Box::new(
            sample_transfer_operation(),
        ))],
        vec![Signature(vec![0; 64])],
    );

    let err = signed_transaction
        .to_fc_bytes()
        .expect_err("invalid signature length propagates through signed transaction");
    assert!(matches!(
        err,
        FcSerializeError::InvalidFixedBytes {
            type_name: "signature",
            expected_len: 65,
            actual_len: 64,
        }
    ));
}

#[test]
fn transaction_fc_propagates_operation_errors() {
    let mut transfer = sample_transfer_operation();
    transfer.memo = Some(MemoData {
        from: "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV".to_string(),
        to: "BTS4tVMTu4hrMTGeAQpAEzueCYqEESJQgkaH9DVJNnzK1mzPCmP45".to_string(),
        nonce: 0,
        message: Vec::new(),
    });
    let transaction =
        sample_transaction_with_operations(vec![Operation::TransferOperation(Box::new(transfer))]);

    let err = transaction
        .to_fc_bytes()
        .expect_err("nested operation error propagates through transaction");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "MemoData",
            reason: "memo FC serialization is not implemented in the minimal transfer slice"
        }
    ));
}

#[test]
fn transaction_signature_preimage_prefixes_chain_id_bytes() {
    let transaction = sample_transaction_with_operations(vec![Operation::TransferOperation(
        Box::new(sample_transfer_operation()),
    )]);

    let mut operation_payload = vec![0];
    operation_payload.extend(expected_transfer_payload());
    let transaction_bytes = expected_transaction_payload(&[operation_payload]);
    let mut expected = expected_swaplock_chain_id_bytes();
    expected.extend(transaction_bytes);

    assert_eq!(
        transaction
            .signature_preimage_bytes()
            .expect("build transaction signature preimage"),
        expected
    );
}

#[test]
fn transaction_fc_matches_bitsharesjs_signature_fixture_transaction_bytes() {
    let transaction = sample_transaction_with_operations(vec![Operation::TransferOperation(
        Box::new(sample_transfer_operation()),
    )]);

    assert_eq!(
        transaction
            .to_fc_bytes()
            .expect("serialize fixture transaction"),
        decode_hex("0100020000000300000001000000000000000000000102a08601000000000000000000")
    );
}

#[test]
fn transaction_signature_digest_hashes_signature_preimage() {
    let transaction = sample_transaction_with_operations(vec![Operation::TransferOperation(
        Box::new(sample_transfer_operation()),
    )]);

    let mut operation_payload = vec![0];
    operation_payload.extend(expected_transfer_payload());
    let transaction_bytes = expected_transaction_payload(&[operation_payload]);
    let mut expected_preimage = expected_swaplock_chain_id_bytes();
    expected_preimage.extend(transaction_bytes);

    assert_eq!(
        transaction
            .signature_digest_bytes()
            .expect("build transaction signature digest"),
        sha256_bytes(&expected_preimage)
    );
}

#[test]
fn transaction_signature_digest_matches_bitsharesjs_signature_fixture_digest() {
    let transaction = sample_transaction_with_operations(vec![Operation::TransferOperation(
        Box::new(sample_transfer_operation()),
    )]);

    assert_eq!(
        transaction
            .signature_preimage_bytes()
            .expect("build fixture signature preimage"),
        decode_hex(
            "f990ce83af5cf2d55c180ca4bd4b34161ccff2b2f8cc7d4987eea9555153930e0100020000000300000001000000000000000000000102a08601000000000000000000",
        )
    );
    let expected_digest: [u8; 32] =
        decode_hex("b459775b7ac4c1f0d1e5988d112846c0a9b75c8b8bbbd613f80499366d9be635")
            .try_into()
            .expect("digest fixture is 32 bytes");
    assert_eq!(
        transaction
            .signature_digest_bytes()
            .expect("build fixture signature digest"),
        expected_digest
    );
}

#[test]
fn transaction_sign_with_wif_matches_bitsharesjs_signature_fixture() {
    let Ok(wif) = std::env::var("SWAPLOCK_ACTIVE_WIF") else {
        eprintln!("skipping signing fixture test because SWAPLOCK_ACTIVE_WIF is not set");
        return;
    };
    let transaction = sample_transaction_with_operations(vec![Operation::TransferOperation(
        Box::new(sample_transfer_operation()),
    )]);

    assert_eq!(
        transaction
            .sign_with_wif(&wif)
            .expect("sign fixture transaction")
            .to_fc_bytes()
            .expect("serialize generated signature"),
        bitsharesjs_signature_vector_bytes()
    );
}

#[test]
fn transaction_signed_with_wif_matches_bitsharesjs_signed_transaction_fixture() {
    let Ok(wif) = std::env::var("SWAPLOCK_ACTIVE_WIF") else {
        eprintln!(
            "skipping signed transaction fixture test because SWAPLOCK_ACTIVE_WIF is not set"
        );
        return;
    };
    let transaction = sample_transaction_with_operations(vec![Operation::TransferOperation(
        Box::new(sample_transfer_operation()),
    )]);

    assert_eq!(
        transaction
            .signed_with_wif(&wif)
            .expect("sign fixture transaction")
            .to_fc_bytes()
            .expect("serialize generated signed transaction"),
        decode_hex(
            "0100020000000300000001000000000000000000000102a08601000000000000000000011f4a8b0c4b54ffd78ee503c2e9112932b2caa726086aa1acb72cc550e1fea0407e4765ad8766314c4e8eb2b42017ac1048ebece448639f63d8082a1e893009a874",
        )
    );
}

#[test]
fn transaction_signed_with_wif_propagates_invalid_wif_without_echoing_secret() {
    let transaction = sample_transaction_with_operations(vec![Operation::TransferOperation(
        Box::new(sample_transfer_operation()),
    )]);
    let secret_like_value = "not-a-wif-secret-like-value";

    let err = transaction
        .signed_with_wif(secret_like_value)
        .expect_err("invalid WIF fails");
    assert!(matches!(
        err,
        FcSerializeError::InvalidPrivateKey {
            reason: "WIF is not valid base58",
        }
    ));
    assert!(!err.to_string().contains(secret_like_value));
}

#[test]
fn transaction_signature_preimage_propagates_operation_errors() {
    let mut transfer = sample_transfer_operation();
    transfer.memo = Some(MemoData {
        from: "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV".to_string(),
        to: "BTS4tVMTu4hrMTGeAQpAEzueCYqEESJQgkaH9DVJNnzK1mzPCmP45".to_string(),
        nonce: 0,
        message: Vec::new(),
    });
    let transaction =
        sample_transaction_with_operations(vec![Operation::TransferOperation(Box::new(transfer))]);

    let err = transaction
        .signature_preimage_bytes()
        .expect_err("nested operation error propagates through signature preimage");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "MemoData",
            reason: "memo FC serialization is not implemented in the minimal transfer slice"
        }
    ));
}

#[test]
fn transaction_signature_digest_propagates_operation_errors() {
    let mut transfer = sample_transfer_operation();
    transfer.memo = Some(MemoData {
        from: "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV".to_string(),
        to: "BTS4tVMTu4hrMTGeAQpAEzueCYqEESJQgkaH9DVJNnzK1mzPCmP45".to_string(),
        nonce: 0,
        message: Vec::new(),
    });
    let transaction =
        sample_transaction_with_operations(vec![Operation::TransferOperation(Box::new(transfer))]);

    let err = transaction
        .signature_digest_bytes()
        .expect_err("nested operation error propagates through signature digest");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "MemoData",
            reason: "memo FC serialization is not implemented in the minimal transfer slice"
        }
    ));
}

#[test]
fn proposal_create_operation_fc_serializes_nested_transfer_operation() {
    assert_eq!(
        sample_proposal_create_operation()
            .to_fc_bytes()
            .expect("serialize proposal create operation"),
        expected_proposal_create_payload()
    );
}

#[test]
fn operation_fc_serializes_proposal_create_tag_and_nested_operation_payload() {
    let bytes = Operation::ProposalCreateOperation(Box::new(sample_proposal_create_operation()))
        .to_fc_bytes()
        .expect("serialize proposal create variant");

    let mut expected = vec![22];
    expected.extend(expected_proposal_create_payload());
    assert_eq!(bytes, expected);
}

#[test]
fn proposal_create_operation_propagates_nested_operation_fc_errors() {
    let mut transfer = sample_transfer_operation();
    transfer.memo = Some(MemoData {
        from: "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV".to_string(),
        to: "BTS4tVMTu4hrMTGeAQpAEzueCYqEESJQgkaH9DVJNnzK1mzPCmP45".to_string(),
        nonce: 0,
        message: Vec::new(),
    });
    let mut proposal = sample_proposal_create_operation();
    proposal.proposed_ops = vec![OpWrapper {
        op: Operation::TransferOperation(Box::new(transfer)),
    }];

    let err = proposal
        .to_fc_bytes()
        .expect_err("nested transfer memo still fails explicitly");

    assert!(matches!(
        err,
        FcSerializeError::UnsupportedValue {
            type_name: "MemoData",
            reason: "memo FC serialization is not implemented in the minimal transfer slice"
        }
    ));
}
