use std::env;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use graphene_chain_swaplock::account_create::{
    account_options, build_account_create_transaction, signed_transaction_json,
    single_key_authority, AccountCreateTransactionInput,
};
use graphene_chain_swaplock::bindings::generated::FcSerialize;
use graphene_chain_swaplock::broadcast::sign_and_broadcast_transaction;
use graphene_chain_swaplock::database_api::{
    account_balance, active_public_key_for_account, head_block, lookup_account_id,
    lookup_account_id_optional, required_fee,
};
use graphene_chain_swaplock::history_api::{
    find_account_history_confirmation, get_account_history, history_entry_matches_account_create,
    AccountCreateConfirmationCriteria, AccountHistoryQuery,
};
use graphene_chain_swaplock::rpc::GrapheneRpc;
use graphene_chain_swaplock::transaction::set_first_operation_fee;
use graphene_chain_swaplock::SwaplockSession;

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = env::var("SWAPLOCK_RPC_URL")?;
    if !rpc_url.starts_with("wss://") && !env_flag("SWAPLOCK_ALLOW_INSECURE_WS") {
        return Err("SWAPLOCK_RPC_URL must use wss://; set SWAPLOCK_ALLOW_INSECURE_WS=1 only for local insecure test nodes".into());
    }
    let wif = env::var("SWAPLOCK_ACTIVE_WIF")?;
    let registrar_account = env::var("SWAPLOCK_ACCOUNT")?;
    let new_account = env::var("SWAPLOCK_NEW_ACCOUNT").unwrap_or_else(|_| unique_account_name());
    if new_account == registrar_account {
        return Err("SWAPLOCK_NEW_ACCOUNT must differ from SWAPLOCK_ACCOUNT".into());
    }
    let asset_id = env::var("SWAPLOCK_ASSET_ID").unwrap_or_else(|_| "1.3.0".to_string());
    let max_fee = env::var("SWAPLOCK_MAX_FEE")
        .ok()
        .map(|value| value.parse::<i64>())
        .transpose()?
        .unwrap_or(1_000_000_000);

    let mut session = SwaplockSession::connect(&rpc_url)?;
    let database_api_id = session.database_api_id();
    if lookup_account_id_optional(session.rpc_mut(), database_api_id, &new_account)?.is_some() {
        return Err(format!("account already exists: {new_account}").into());
    }

    let registrar_id = lookup_account_id(session.rpc_mut(), database_api_id, &registrar_account)?;
    let registrar_signing_public_key = match env::var("SWAPLOCK_ACTIVE_PUBLIC_KEY") {
        Ok(public_key) => public_key,
        Err(env::VarError::NotPresent) => active_public_key_for_account(session.rpc_mut(), database_api_id, &registrar_id)?.ok_or("could not determine a single registrar active public key; set SWAPLOCK_ACTIVE_PUBLIC_KEY")?,
        Err(err) => return Err(err.into()),
    };
    let new_account_public_key = match env::var("SWAPLOCK_NEW_ACCOUNT_PUBLIC_KEY") {
        Ok(public_key) => public_key,
        Err(env::VarError::NotPresent) => registrar_signing_public_key.clone(),
        Err(err) => return Err(err.into()),
    };

    let head = head_block(session.rpc_mut(), database_api_id)?;
    let head_block_number = head.number;
    let header =
        open_graphene_sdk_core::transaction_header_from_head(&head, Duration::from_secs(60))?;

    let mut transaction = build_account_create_transaction(AccountCreateTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        expiration: header.expiration,
        fee_amount: 0,
        fee_asset_id: asset_id.clone(),
        registrar_id: registrar_id.clone(),
        referrer_id: registrar_id.clone(),
        referrer_percent: 10_000,
        name: new_account.clone(),
        owner: single_key_authority(new_account_public_key.clone()),
        active: single_key_authority(new_account_public_key.clone()),
        options: account_options(new_account_public_key.clone(), registrar_id.clone()),
        extensions: None,
    });

    let fee = required_fee(
        session.rpc_mut(),
        database_api_id,
        &transaction,
        &asset_id,
        signed_transaction_json,
    )?;
    if fee.amount > max_fee {
        return Err(format!(
            "required fee {} exceeds SWAPLOCK_MAX_FEE {}; refusing to sign",
            fee.amount, max_fee
        )
        .into());
    }
    set_first_operation_fee(&mut transaction, fee.clone())?;
    let balance_before = account_balance(
        session.rpc_mut(),
        database_api_id,
        &registrar_id,
        &fee.asset_id.0,
    )?;
    if balance_before < fee.amount {
        return Err(format!(
            "insufficient balance for fee asset {}: balance {}, required {}",
            fee.asset_id.0, balance_before, fee.amount
        )
        .into());
    }

    println!("Creating account {new_account} with registrar {registrar_account}");
    println!("Fee: {} asset {}", fee.amount, fee.asset_id.0);
    println!("Digest: {}", hex(&transaction.signature_digest_bytes()?));
    if env_flag("SWAPLOCK_DEBUG") {
        println!("debug_new_account_public_key: {new_account_public_key}");
        println!("debug_registrar_signing_public_key: {registrar_signing_public_key}");
        println!("debug_registrar_id: {registrar_id}");
        println!("debug_head_block_number: {head_block_number}");
        println!(
            "debug_transaction_hex: {}",
            hex(&transaction.to_fc_bytes()?)
        );
    }

    let network_broadcast_api_id = session.network_broadcast_api_id()?;
    let history_api_id = session.history_api_id()?;
    sign_and_broadcast_transaction(
        session.rpc_mut(),
        network_broadcast_api_id,
        &transaction,
        &wif,
        &registrar_signing_public_key,
        signed_transaction_json,
    )?;
    println!("Broadcast: submitted");

    let created_id = wait_for_account(session.rpc_mut(), database_api_id, &new_account)?
        .ok_or("broadcast submitted but new account was not found")?;
    println!("Created: {new_account} ({created_id})");
    let criteria = AccountCreateConfirmationCriteria {
        account_name: &new_account,
        min_block_num: head_block_number,
    };
    let history = get_account_history(
        session.rpc_mut(),
        history_api_id,
        &AccountHistoryQuery::recent(&registrar_id),
    )?;
    if let Some(confirmation) = find_account_history_confirmation(&history, |entry| {
        history_entry_matches_account_create(entry, &criteria)
    })? {
        println!(
            "Confirmed: block {}, history {}",
            confirmation.block_num, confirmation.id
        );
    }

    Ok(())
}

fn wait_for_account(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let attempts = confirm_attempts()?;
    let delay = confirm_delay()?;
    for attempt in 0..attempts {
        if let Some(account_id) = lookup_account_id_optional(rpc, api_id, account_name)? {
            return Ok(Some(account_id));
        }
        if attempt + 1 < attempts {
            std::thread::sleep(delay);
        }
    }
    Ok(None)
}

fn unique_account_name() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before epoch")
        .as_secs();
    format!("og-test-{seconds}")
}

fn confirm_attempts() -> Result<u64, Box<dyn Error>> {
    Ok(env::var("SWAPLOCK_CONFIRM_ATTEMPTS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(10))
}

fn confirm_delay() -> Result<Duration, Box<dyn Error>> {
    Ok(Duration::from_millis(
        env::var("SWAPLOCK_CONFIRM_DELAY_MS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(2_000),
    ))
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
