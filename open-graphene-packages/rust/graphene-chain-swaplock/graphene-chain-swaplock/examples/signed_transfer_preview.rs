use std::env;
use std::error::Error;
use std::time::Duration;

use graphene_chain_swaplock::bindings::generated::FcSerialize;
use graphene_chain_swaplock::history_api::{
    history_entry_matches_transfer, wait_for_account_history_confirmation, AccountHistoryQuery,
    HistoryPollConfig, TransferConfirmationCriteria,
};
use graphene_chain_swaplock::network_broadcast_api::broadcast_transaction;
use graphene_chain_swaplock::signing::sign_transaction_checked;
use graphene_chain_swaplock::transaction::set_first_operation_fee;
use graphene_chain_swaplock::transfer::{
    build_transfer_transaction, signed_transaction_json, TransferTransactionInput,
};
use graphene_chain_swaplock::SwaplockSession;

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = env::var("SWAPLOCK_RPC_URL")?;
    if !rpc_url.starts_with("wss://") && !env_flag("SWAPLOCK_ALLOW_INSECURE_WS") {
        return Err(
            "SWAPLOCK_RPC_URL must use wss://; set SWAPLOCK_ALLOW_INSECURE_WS=1 only for local insecure test nodes"
                .into(),
        );
    }
    let wif = env::var("SWAPLOCK_ACTIVE_WIF")?;
    let from_account = env::var("SWAPLOCK_ACCOUNT")?;
    let to_account = transfer_recipient_account(&from_account)?;
    let asset_id = env::var("SWAPLOCK_ASSET_ID").unwrap_or_else(|_| "1.3.0".to_string());
    let max_fee = env::var("SWAPLOCK_MAX_FEE")
        .ok()
        .map(|value| value.parse::<i64>())
        .transpose()?
        .unwrap_or(1_000_000);
    let human_amount = env::var("SWAPLOCK_TRANSFER_AMOUNT").unwrap_or_else(|_| "1".to_string());

    let mut session = SwaplockSession::connect(&rpc_url)?;
    let asset_precision = asset_precision(&mut session, &asset_id)?;
    let amount = transfer_amount_raw(&human_amount, asset_precision)?;

    let from_id = session.lookup_account_id(&from_account)?;
    let to_id = session.lookup_account_id(&to_account)?;
    let expected_public_key = match env::var("SWAPLOCK_ACTIVE_PUBLIC_KEY") {
        Ok(public_key) => Some(("env", public_key)),
        Err(env::VarError::NotPresent) => session
            .active_public_key_for_account(&from_id)?
            .map(|public_key| ("chain_active_authority", public_key)),
        Err(err) => return Err(err.into()),
    };
    let head = session.head_block()?;
    let head_block_number = head.number;
    let header =
        open_graphene_sdk_core::transaction_header_from_head(&head, Duration::from_secs(60))?;
    let expiration = header.expiration;
    let ref_block_num = header.ref_block_num;
    let ref_block_prefix = header.ref_block_prefix;

    let mut transaction = build_transfer_transaction(TransferTransactionInput {
        ref_block_num,
        ref_block_prefix,
        expiration,
        from_id: from_id.clone(),
        to_id: to_id.clone(),
        asset_id: asset_id.clone(),
        amount,
        fee_amount: 0,
        fee_asset_id: asset_id.clone(),
    });

    let fee = session.required_fee(&transaction, &asset_id, signed_transaction_json)?;
    let expected_fee_amount = fee.amount;
    if fee.amount > max_fee {
        return Err(format!(
            "required fee {} exceeds SWAPLOCK_MAX_FEE {}; refusing to sign",
            fee.amount, max_fee
        )
        .into());
    }
    let balance_before = session.account_balance(&from_id, &asset_id)?;
    open_graphene_sdk_core::ensure_sufficient_balance(&open_graphene_sdk_core::BalanceCheck {
        balance: open_graphene_sdk_core::AssetAmount {
            amount: balance_before,
            asset_id: open_graphene_sdk_core::AssetIdRef::parse(&asset_id)?,
        },
        transfer_amount: open_graphene_sdk_core::AssetAmount {
            amount,
            asset_id: open_graphene_sdk_core::AssetIdRef::parse(&asset_id)?,
        },
        fee: open_graphene_sdk_core::AssetAmount {
            amount: fee.amount,
            asset_id: open_graphene_sdk_core::AssetIdRef::parse(&fee.asset_id.0)?,
        },
    })?;
    set_first_operation_fee(&mut transaction, fee.clone())?;

    let signed_transaction = sign_transaction_checked(
        &transaction,
        &wif,
        expected_public_key
            .as_ref()
            .map(|(_, public_key)| public_key.as_str())
            .ok_or("signature public key verification is required before broadcast")?,
    )?;
    let signature_public_key_match = expected_public_key
        .as_ref()
        .map(|(source, _)| (*source, true));

    let digest_hex = hex(&transaction.signature_digest_bytes()?);
    ensure_signature_public_key_match(signature_public_key_match.map(|(_, matches)| matches))?;

    println!(
        "Sending {} asset {} from {} to {}",
        open_graphene_sdk_core::format_raw_amount(amount, asset_precision),
        asset_id,
        from_account,
        to_account
    );
    println!(
        "Fee: {} asset {}",
        open_graphene_sdk_core::format_raw_amount(expected_fee_amount, asset_precision),
        asset_id
    );
    println!(
        "Balance: {} -> {} asset {}",
        open_graphene_sdk_core::format_raw_amount(balance_before, asset_precision),
        open_graphene_sdk_core::format_raw_amount(
            balance_before - amount - expected_fee_amount,
            asset_precision
        ),
        asset_id
    );
    println!("Digest: {digest_hex}");

    if env_flag("SWAPLOCK_DEBUG") {
        println!("debug_read_only: false");
        println!("debug_broadcast: true");
        println!("debug_from_id: {from_id}");
        println!("debug_asset_precision: {asset_precision}");
        println!("debug_amount_raw: {amount}");
        println!("debug_fee_raw: {}", expected_fee_amount);
        println!("debug_balance_before_raw: {balance_before}");
        println!("debug_head_block_number: {head_block_number}");
        println!("debug_ref_block_num: {ref_block_num}");
        println!("debug_ref_block_prefix: {ref_block_prefix}");
        if let Some((source, matches_expected_public_key)) = signature_public_key_match {
            println!("debug_signature_public_key_source: {source}");
            println!("debug_signature_public_key_matches: {matches_expected_public_key}");
        } else {
            println!(
                "debug_signature_public_key_matches: <skipped; no single active public key found>"
            );
        }
        println!(
            "debug_transaction_hex: {}",
            hex(&transaction.to_fc_bytes()?)
        );
        if env_flag("SWAPLOCK_PRINT_SIGNED_TX") {
            println!(
                "debug_signed_transaction_hex: {}",
                hex(&signed_transaction.to_fc_bytes()?)
            );
        } else {
            println!(
                "debug_signed_transaction_hex: <hidden; set SWAPLOCK_PRINT_SIGNED_TX=1 to print>"
            );
        }
    }

    let network_broadcast_api_id = session.network_broadcast_api_id()?;
    let history_api_id = session.history_api_id()?;
    broadcast_transaction(
        session.rpc_mut(),
        network_broadcast_api_id,
        signed_transaction_json(&signed_transaction)?,
    )?;
    println!("Broadcast: submitted");

    let criteria = TransferConfirmationCriteria {
        from_id: &from_id,
        to_id: &to_id,
        amount,
        asset_id: &asset_id,
        fee_amount: expected_fee_amount,
        min_block_num: head_block_number,
    };
    let confirmation = wait_for_account_history_confirmation(
        session.rpc_mut(),
        history_api_id,
        &AccountHistoryQuery::recent(&from_id),
        &confirm_poll_config()?,
        |entry| history_entry_matches_transfer(entry, &criteria),
    )?
    .ok_or("broadcast submitted but transfer was not found in account history")?;
    println!(
        "Confirmed: block {}, history {}",
        confirmation.block_num, confirmation.id
    );
    if env_flag("SWAPLOCK_DEBUG") {
        println!(
            "debug_confirmation_trx_in_block: {}",
            confirmation
                .trx_in_block
                .ok_or("history entry missing trx_in_block")?
        );
        println!(
            "debug_confirmation_op_in_trx: {}",
            confirmation
                .op_in_trx
                .ok_or("history entry missing op_in_trx")?
        );
        println!(
            "debug_confirmation_virtual_op: {}",
            confirmation
                .virtual_op
                .ok_or("history entry missing virtual_op")?
        );
    }

    Ok(())
}

fn confirm_poll_config() -> Result<HistoryPollConfig, Box<dyn Error>> {
    let attempts = env::var("SWAPLOCK_CONFIRM_ATTEMPTS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(10);
    let delay = Duration::from_millis(
        env::var("SWAPLOCK_CONFIRM_DELAY_MS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(2_000),
    );

    Ok(HistoryPollConfig { attempts, delay })
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn transfer_recipient_account(from_account: &str) -> Result<String, Box<dyn Error>> {
    let to_account = env::var("SWAPLOCK_TO_ACCOUNT")?;
    if to_account == from_account {
        return Err(
            "SWAPLOCK_TO_ACCOUNT must differ from SWAPLOCK_ACCOUNT for transfer_operation".into(),
        );
    }
    Ok(to_account)
}

fn ensure_signature_public_key_match(
    signature_public_key_matches: Option<bool>,
) -> std::result::Result<(), &'static str> {
    match signature_public_key_matches {
        Some(true) => Ok(()),
        Some(false) => Err("signature public key verification failed; refusing to broadcast"),
        None => Err("signature public key verification is required before broadcast"),
    }
}

fn asset_precision(session: &mut SwaplockSession, asset_id: &str) -> Result<u8, Box<dyn Error>> {
    let asset = session
        .asset_object(asset_id)?
        .ok_or("asset object was not returned")?;
    Ok(asset.precision)
}

fn transfer_amount_raw(human_amount: &str, precision: u8) -> Result<i64, Box<dyn Error>> {
    if let Ok(raw_amount) = env::var("SWAPLOCK_TRANSFER_RAW_AMOUNT") {
        return Ok(raw_amount.parse()?);
    }
    open_graphene_sdk_core::decimal_to_raw_amount(human_amount, precision).map_err(Into::into)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn signature_public_key_guard_requires_verified_match() {
        assert_eq!(ensure_signature_public_key_match(Some(true)), Ok(()));
        assert_eq!(
            ensure_signature_public_key_match(Some(false)),
            Err("signature public key verification failed; refusing to broadcast")
        );
        assert_eq!(
            ensure_signature_public_key_match(None),
            Err("signature public key verification is required before broadcast")
        );
    }

    #[test]
    fn transfer_recipient_must_not_equal_sender() {
        unsafe {
            env::set_var("SWAPLOCK_TO_ACCOUNT", "swaplock");
        }
        let err = transfer_recipient_account("swaplock").expect_err("self-transfer fails locally");
        assert_eq!(
            err.to_string(),
            "SWAPLOCK_TO_ACCOUNT must differ from SWAPLOCK_ACCOUNT for transfer_operation"
        );
        unsafe {
            env::remove_var("SWAPLOCK_TO_ACCOUNT");
        }
    }

    #[test]
    fn account_history_confirmation_matches_transfer_fields() {
        let history = json!([
            {
                "id": "1.11.22",
                "block_num": 123,
                "trx_in_block": 1,
                "op_in_trx": 0,
                "virtual_op": 0,
                "op": [0, {
                    "fee": { "amount": 10, "asset_id": "1.3.0" },
                    "from": "1.2.100",
                    "to": "1.2.0",
                    "amount": { "amount": 100000, "asset_id": "1.3.0" },
                    "memo": null,
                    "extensions": []
                }]
            }
        ]);
        let confirmation = graphene_chain_swaplock::history_api::find_account_history_confirmation(
            &history,
            |entry| {
                history_entry_matches_transfer(
                    entry,
                    &TransferConfirmationCriteria {
                        from_id: "1.2.100",
                        to_id: "1.2.0",
                        amount: 100000,
                        asset_id: "1.3.0",
                        fee_amount: 10,
                        min_block_num: 123,
                    },
                )
            },
        )
        .unwrap()
        .expect("matching transfer is confirmed");

        assert_eq!(confirmation.id, "1.11.22");
        assert_eq!(confirmation.block_num, 123);
        assert_eq!(confirmation.trx_in_block.unwrap(), 1);
        assert_eq!(confirmation.op_in_trx.unwrap(), 0);
        assert_eq!(confirmation.virtual_op.unwrap(), 0);
    }

    #[test]
    fn account_history_confirmation_rejects_wrong_amount() {
        let history = json!([
            {
                "id": "1.11.22",
                "block_num": 123,
                "trx_in_block": 1,
                "op_in_trx": 0,
                "virtual_op": 0,
                "op": [0, {
                    "fee": { "amount": 10, "asset_id": "1.3.0" },
                    "from": "1.2.100",
                    "to": "1.2.0",
                    "amount": { "amount": 1, "asset_id": "1.3.0" },
                    "memo": null,
                    "extensions": []
                }]
            }
        ]);

        assert!(
            graphene_chain_swaplock::history_api::find_account_history_confirmation(
                &history,
                |entry| history_entry_matches_transfer(
                    entry,
                    &TransferConfirmationCriteria {
                        from_id: "1.2.100",
                        to_id: "1.2.0",
                        amount: 100000,
                        asset_id: "1.3.0",
                        fee_amount: 10,
                        min_block_num: 123,
                    }
                )
            )
            .unwrap()
            .is_none()
        );
    }
}
