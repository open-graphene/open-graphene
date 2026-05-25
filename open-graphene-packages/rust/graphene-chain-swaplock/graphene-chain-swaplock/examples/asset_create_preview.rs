use std::env;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use graphene_chain_swaplock::asset_create::{
    build_asset_create_transaction, signed_transaction_json, AssetCreateTransactionInput,
};
use graphene_chain_swaplock::bindings::generated::FcSerialize;
use graphene_chain_swaplock::broadcast::sign_and_broadcast_transaction;
use graphene_chain_swaplock::database_api::{
    account_balance, active_public_key_for_account, head_block, lookup_account_id,
    lookup_asset_id_optional, required_fee,
};
use graphene_chain_swaplock::rpc::GrapheneRpc;
use graphene_chain_swaplock::transaction::set_first_operation_fee;

fn main() -> Result<(), Box<dyn Error>> {
    let rpc_url = env::var("SWAPLOCK_RPC_URL")?;
    if !rpc_url.starts_with("wss://") && !env_flag("SWAPLOCK_ALLOW_INSECURE_WS") {
        return Err("SWAPLOCK_RPC_URL must use wss://; set SWAPLOCK_ALLOW_INSECURE_WS=1 only for local insecure test nodes".into());
    }
    let wif = env::var("SWAPLOCK_ACTIVE_WIF")?;
    let issuer_account = env::var("SWAPLOCK_ACCOUNT")?;
    let symbol = env::var("SWAPLOCK_ASSET_SYMBOL").unwrap_or_else(|_| unique_asset_symbol());
    validate_asset_symbol(&symbol)?;
    let fee_asset_id = env::var("SWAPLOCK_ASSET_ID").unwrap_or_else(|_| "1.3.0".to_string());
    let precision = env::var("SWAPLOCK_ASSET_PRECISION")
        .ok()
        .map(|value| value.parse::<u8>())
        .transpose()?
        .unwrap_or(5);
    let max_supply = env::var("SWAPLOCK_ASSET_MAX_SUPPLY")
        .ok()
        .map(|value| value.parse::<i64>())
        .transpose()?
        .unwrap_or(1_000_000_000_000_000);
    let description = env::var("SWAPLOCK_ASSET_DESCRIPTION")
        .unwrap_or_else(|_| "open-graphene live asset_create proof".to_string());
    let max_fee = env::var("SWAPLOCK_MAX_FEE")
        .ok()
        .map(|value| value.parse::<i64>())
        .transpose()?
        .unwrap_or(1_000_000_000);

    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;
    if lookup_asset_id_optional(&mut rpc, database_api_id, &symbol)?.is_some() {
        return Err(format!("asset symbol already exists: {symbol}").into());
    }

    let issuer_id = lookup_account_id(&mut rpc, database_api_id, &issuer_account)?;
    let issuer_signing_public_key =
        active_public_key_for_account(&mut rpc, database_api_id, &issuer_id)?
            .ok_or("could not determine a single issuer active public key from chain")?;
    if let Ok(env_public_key) = env::var("SWAPLOCK_ACTIVE_PUBLIC_KEY") {
        if env_public_key != issuer_signing_public_key {
            return Err(
                "SWAPLOCK_ACTIVE_PUBLIC_KEY does not match the issuer active authority on chain"
                    .into(),
            );
        }
    }

    let head = head_block(&mut rpc, database_api_id)?;
    let head_block_number = head.number;
    let header =
        open_graphene_sdk_core::transaction_header_from_head(&head, Duration::from_secs(60))?;

    let mut transaction = build_asset_create_transaction(AssetCreateTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        expiration: header.expiration,
        fee_amount: 0,
        fee_asset_id: fee_asset_id.clone(),
        issuer_id: issuer_id.clone(),
        symbol: symbol.clone(),
        precision,
        max_supply,
        description,
    });

    let fee = required_fee(
        &mut rpc,
        database_api_id,
        &transaction,
        &fee_asset_id,
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
    let balance_before = account_balance(&mut rpc, database_api_id, &issuer_id, &fee.asset_id.0)?;
    if balance_before < fee.amount {
        return Err(format!(
            "insufficient balance for fee asset {}: balance {}, required {}",
            fee.asset_id.0, balance_before, fee.amount
        )
        .into());
    }

    println!("Creating asset {symbol} with issuer {issuer_account}");
    println!("Fee: {} asset {}", fee.amount, fee.asset_id.0);
    println!("Digest: {}", hex(&transaction.signature_digest_bytes()?));
    if env_flag("SWAPLOCK_DEBUG") {
        println!("debug_issuer_signing_public_key: {issuer_signing_public_key}");
        println!("debug_issuer_id: {issuer_id}");
        println!("debug_head_block_number: {head_block_number}");
        println!(
            "debug_transaction_hex: {}",
            hex(&transaction.to_fc_bytes()?)
        );
    }

    let network_broadcast_api_id = rpc.network_broadcast_api_id()?;
    sign_and_broadcast_transaction(
        &mut rpc,
        network_broadcast_api_id,
        &transaction,
        &wif,
        &issuer_signing_public_key,
        signed_transaction_json,
    )?;
    println!("Broadcast: submitted");

    let created_id = wait_for_asset(&mut rpc, database_api_id, &symbol)?
        .ok_or("broadcast submitted but new asset was not found")?;
    println!("Created asset: {symbol} ({created_id})");

    Ok(())
}

fn wait_for_asset(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let attempts = confirm_attempts()?;
    let delay = confirm_delay()?;
    for attempt in 0..attempts {
        if let Some(asset_id) = lookup_asset_id_optional(rpc, api_id, symbol)? {
            return Ok(Some(asset_id));
        }
        if attempt + 1 < attempts {
            std::thread::sleep(delay);
        }
    }
    Ok(None)
}

fn unique_asset_symbol() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before epoch")
        .as_secs();
    format!("OGT{seconds}")
}

fn validate_asset_symbol(symbol: &str) -> Result<(), Box<dyn Error>> {
    if !(3..=16).contains(&symbol.len()) {
        return Err("SWAPLOCK_ASSET_SYMBOL must be between 3 and 16 characters".into());
    }
    if !symbol
        .chars()
        .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
    {
        return Err(
            "SWAPLOCK_ASSET_SYMBOL must use only uppercase ASCII letters and digits".into(),
        );
    }
    if !symbol
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
    {
        return Err("SWAPLOCK_ASSET_SYMBOL must start with an uppercase ASCII letter".into());
    }
    Ok(())
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
