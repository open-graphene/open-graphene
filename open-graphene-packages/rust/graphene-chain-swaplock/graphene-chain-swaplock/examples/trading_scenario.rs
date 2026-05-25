use std::env;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use graphene_chain_swaplock::account_create::{
    account_options, build_account_create_transaction,
    signed_transaction_json as account_create_json, single_key_authority,
    AccountCreateTransactionInput,
};
use graphene_chain_swaplock::asset_create::{
    build_asset_create_transaction, signed_transaction_json as asset_create_json,
    AssetCreateTransactionInput,
};
use graphene_chain_swaplock::asset_issue::{
    build_asset_issue_transaction, signed_transaction_json as asset_issue_json,
    AssetIssueTransactionInput,
};
use graphene_chain_swaplock::bindings::generated::types::{SignedTransaction, Transaction};
use graphene_chain_swaplock::broadcast::sign_and_broadcast_transaction;
use graphene_chain_swaplock::database_api::{
    account_balance, active_public_key_for_account, lookup_account_id, lookup_account_id_optional,
    lookup_asset_id_optional, next_transaction_header, wait_for_account, wait_for_asset,
    wait_for_balance_at_least, wait_for_limit_order, wait_for_order_gone,
};
use graphene_chain_swaplock::limit_order_cancel::{
    build_limit_order_cancel_transaction, signed_transaction_json as limit_order_cancel_json,
    LimitOrderCancelTransactionInput,
};
use graphene_chain_swaplock::limit_order_create::{
    build_limit_order_create_transaction, signed_transaction_json as limit_order_create_json,
    LimitOrderCreateTransactionInput,
};
use graphene_chain_swaplock::rpc::GrapheneRpc;
use graphene_chain_swaplock::transaction::apply_required_fee;
use graphene_chain_swaplock::transfer::{
    build_transfer_transaction, signed_transaction_json as transfer_json, TransferTransactionInput,
};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TradingScenarioResult {
    pub registrar_id: String,
    pub account_a: String,
    pub account_b: String,
    pub account_a_id: String,
    pub account_b_id: String,
    pub asset_a_symbol: String,
    pub asset_b_symbol: String,
    pub asset_a_id: String,
    pub asset_b_id: String,
    pub opened_order_id: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let result = run_trading_scenario_from_env()?;
    println!(
        "Trading scenario result: accounts {} ({}) / {} ({}), assets {} ({}) / {} ({}), order {} canceled",
        result.account_a,
        result.account_a_id,
        result.account_b,
        result.account_b_id,
        result.asset_a_symbol,
        result.asset_a_id,
        result.asset_b_symbol,
        result.asset_b_id,
        result.opened_order_id
    );
    Ok(())
}

fn run_trading_scenario_from_env() -> Result<TradingScenarioResult, Box<dyn Error>> {
    let rpc_url = env::var("SWAPLOCK_RPC_URL")?;
    if !rpc_url.starts_with("wss://") && !env_flag("SWAPLOCK_ALLOW_INSECURE_WS") {
        return Err("SWAPLOCK_RPC_URL must use wss://; set SWAPLOCK_ALLOW_INSECURE_WS=1 only for local insecure test nodes".into());
    }

    let wif = env::var("SWAPLOCK_ACTIVE_WIF")?;
    let registrar_account = env::var("SWAPLOCK_ACCOUNT")?;
    let fee_asset_id = env::var("SWAPLOCK_ASSET_ID").unwrap_or_else(|_| "1.3.0".to_string());
    let max_fee = env_i64("SWAPLOCK_MAX_FEE", 1_000_000_000)?;
    let account_core_funding = env_i64("SWAPLOCK_SCENARIO_CORE_FUNDING", 50_000_000)?;
    let issue_amount = env_i64("SWAPLOCK_SCENARIO_ISSUE_AMOUNT", 1_000_000)?;
    let order_sell_amount = env_i64("SWAPLOCK_SCENARIO_ORDER_SELL_AMOUNT", 100_000)?;
    let order_receive_amount = env_i64("SWAPLOCK_SCENARIO_ORDER_RECEIVE_AMOUNT", 10_000_000_000)?;

    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;
    let network_broadcast_api_id = rpc.network_broadcast_api_id()?;

    let registrar_id = lookup_account_id(&mut rpc, database_api_id, &registrar_account)?;
    let signing_public_key =
        active_public_key_for_account(&mut rpc, database_api_id, &registrar_id)?
            .ok_or("could not determine registrar active public key from chain")?;
    if let Ok(env_public_key) = env::var("SWAPLOCK_ACTIVE_PUBLIC_KEY") {
        if env_public_key != signing_public_key {
            return Err(
                "SWAPLOCK_ACTIVE_PUBLIC_KEY does not match registrar active authority on chain"
                    .into(),
            );
        }
    }

    let suffix = timestamp_suffix();
    let account_a =
        env::var("SWAPLOCK_SCENARIO_ACCOUNT_A").unwrap_or_else(|_| format!("og-a-{suffix}"));
    let account_b =
        env::var("SWAPLOCK_SCENARIO_ACCOUNT_B").unwrap_or_else(|_| format!("og-b-{suffix}"));
    let asset_a_symbol = env::var("SWAPLOCK_SCENARIO_ASSET_A")
        .unwrap_or_else(|_| format!("OGA{}", suffix_tail(&suffix)));
    let asset_b_symbol = env::var("SWAPLOCK_SCENARIO_ASSET_B")
        .unwrap_or_else(|_| format!("OGB{}", suffix_tail(&suffix)));

    println!("Scenario: registrar={registrar_account} ({registrar_id})");
    println!("Scenario accounts: {account_a}, {account_b}");
    println!("Scenario assets: {asset_a_symbol}, {asset_b_symbol}");

    create_account_if_missing(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &registrar_id,
        &account_a,
    )?;
    create_account_if_missing(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &registrar_id,
        &account_b,
    )?;

    let account_a_id = wait_for_account(&mut rpc, database_api_id, &account_a)?
        .ok_or("account A was not found after creation")?;
    let account_b_id = wait_for_account(&mut rpc, database_api_id, &account_b)?
        .ok_or("account B was not found after creation")?;

    create_asset_if_missing(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &registrar_id,
        &asset_a_symbol,
    )?;
    create_asset_if_missing(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &registrar_id,
        &asset_b_symbol,
    )?;

    let asset_a_id = wait_for_asset(&mut rpc, database_api_id, &asset_a_symbol)?
        .ok_or("asset A was not found after creation")?;
    let asset_b_id = wait_for_asset(&mut rpc, database_api_id, &asset_b_symbol)?
        .ok_or("asset B was not found after creation")?;

    fund_core(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &registrar_id,
        &account_a_id,
        account_core_funding,
    )?;
    fund_core(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &registrar_id,
        &account_b_id,
        account_core_funding,
    )?;

    issue_asset(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &registrar_id,
        &account_a_id,
        &asset_a_id,
        issue_amount,
    )?;
    issue_asset(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &registrar_id,
        &account_b_id,
        &asset_b_id,
        issue_amount,
    )?;

    wait_for_balance_at_least(
        &mut rpc,
        database_api_id,
        &account_a_id,
        &asset_a_id,
        issue_amount,
    )?;
    wait_for_balance_at_least(
        &mut rpc,
        database_api_id,
        &account_b_id,
        &asset_b_id,
        issue_amount,
    )?;

    let order_id = create_unmatched_order(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &account_a_id,
        &asset_a_id,
        order_sell_amount,
        &asset_b_id,
        order_receive_amount,
    )?;
    println!("Opened limit order: {order_id}");

    cancel_order(
        &mut rpc,
        database_api_id,
        network_broadcast_api_id,
        &wif,
        &signing_public_key,
        &fee_asset_id,
        max_fee,
        &account_a_id,
        &order_id,
    )?;
    wait_for_order_gone(&mut rpc, database_api_id, &order_id)?;

    println!("Scenario complete: created accounts/assets, issued balances, opened and canceled {order_id}");
    Ok(TradingScenarioResult {
        registrar_id,
        account_a,
        account_b,
        account_a_id,
        account_b_id,
        asset_a_symbol,
        asset_b_symbol,
        asset_a_id,
        asset_b_id,
        opened_order_id: order_id,
    })
}

fn create_account_if_missing(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    network_broadcast_api_id: u64,
    wif: &str,
    signing_public_key: &str,
    fee_asset_id: &str,
    max_fee: i64,
    registrar_id: &str,
    account_name: &str,
) -> Result<(), Box<dyn Error>> {
    if lookup_account_id_optional(rpc, database_api_id, account_name)?.is_some() {
        println!("Account exists: {account_name}");
        return Ok(());
    }

    let header = next_transaction_header(rpc, database_api_id, Duration::from_secs(60))?;
    let mut transaction = build_account_create_transaction(AccountCreateTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        expiration: header.expiration,
        fee_amount: 0,
        fee_asset_id: fee_asset_id.to_string(),
        registrar_id: registrar_id.to_string(),
        referrer_id: registrar_id.to_string(),
        referrer_percent: 10_000,
        name: account_name.to_string(),
        owner: single_key_authority(signing_public_key.to_string()),
        active: single_key_authority(signing_public_key.to_string()),
        options: account_options(signing_public_key.to_string(), registrar_id.to_string()),
        extensions: None,
    });
    apply_required_fee(
        rpc,
        database_api_id,
        &mut transaction,
        fee_asset_id,
        max_fee,
        account_create_json,
    )?;
    sign_and_broadcast(
        rpc,
        network_broadcast_api_id,
        transaction,
        wif,
        signing_public_key,
        account_create_json,
        &format!("account_create {account_name}"),
    )?;
    wait_for_account(rpc, database_api_id, account_name)?.ok_or("created account was not found")?;
    Ok(())
}

fn create_asset_if_missing(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    network_broadcast_api_id: u64,
    wif: &str,
    signing_public_key: &str,
    fee_asset_id: &str,
    max_fee: i64,
    issuer_id: &str,
    symbol: &str,
) -> Result<(), Box<dyn Error>> {
    if lookup_asset_id_optional(rpc, database_api_id, symbol)?.is_some() {
        println!("Asset exists: {symbol}");
        return Ok(());
    }

    let header = next_transaction_header(rpc, database_api_id, Duration::from_secs(60))?;
    let mut transaction = build_asset_create_transaction(AssetCreateTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        expiration: header.expiration,
        fee_amount: 0,
        fee_asset_id: fee_asset_id.to_string(),
        issuer_id: issuer_id.to_string(),
        symbol: symbol.to_string(),
        precision: 5,
        max_supply: 1_000_000_000_000_000,
        description: "open-graphene trading scenario asset".to_string(),
    });
    apply_required_fee(
        rpc,
        database_api_id,
        &mut transaction,
        fee_asset_id,
        max_fee,
        asset_create_json,
    )?;
    sign_and_broadcast(
        rpc,
        network_broadcast_api_id,
        transaction,
        wif,
        signing_public_key,
        asset_create_json,
        &format!("asset_create {symbol}"),
    )?;
    wait_for_asset(rpc, database_api_id, symbol)?.ok_or("created asset was not found")?;
    Ok(())
}

fn fund_core(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    network_broadcast_api_id: u64,
    wif: &str,
    signing_public_key: &str,
    fee_asset_id: &str,
    max_fee: i64,
    from_id: &str,
    to_id: &str,
    amount: i64,
) -> Result<(), Box<dyn Error>> {
    let before = account_balance(rpc, database_api_id, to_id, fee_asset_id)?;
    if before >= amount {
        println!("Core funding already present for {to_id}: {before} {fee_asset_id}");
        return Ok(());
    }

    let header = next_transaction_header(rpc, database_api_id, Duration::from_secs(60))?;
    let mut transaction = build_transfer_transaction(TransferTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        expiration: header.expiration,
        from_id: from_id.to_string(),
        to_id: to_id.to_string(),
        asset_id: fee_asset_id.to_string(),
        amount,
        fee_amount: 0,
        fee_asset_id: fee_asset_id.to_string(),
    });
    apply_required_fee(
        rpc,
        database_api_id,
        &mut transaction,
        fee_asset_id,
        max_fee,
        transfer_json,
    )?;
    sign_and_broadcast(
        rpc,
        network_broadcast_api_id,
        transaction,
        wif,
        signing_public_key,
        transfer_json,
        &format!("transfer core funding to {to_id}"),
    )?;
    wait_for_balance_at_least(rpc, database_api_id, to_id, fee_asset_id, before + amount)?;
    Ok(())
}

fn issue_asset(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    network_broadcast_api_id: u64,
    wif: &str,
    signing_public_key: &str,
    fee_asset_id: &str,
    max_fee: i64,
    issuer_id: &str,
    recipient_id: &str,
    asset_id: &str,
    amount: i64,
) -> Result<(), Box<dyn Error>> {
    let before = account_balance(rpc, database_api_id, recipient_id, asset_id)?;
    if before >= amount {
        println!("Asset funding already present for {recipient_id}: {before} {asset_id}");
        return Ok(());
    }

    let header = next_transaction_header(rpc, database_api_id, Duration::from_secs(60))?;
    let mut transaction = build_asset_issue_transaction(AssetIssueTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        expiration: header.expiration,
        issuer_id: issuer_id.to_string(),
        issue_to_account_id: recipient_id.to_string(),
        asset_id: asset_id.to_string(),
        amount,
        fee_amount: 0,
        fee_asset_id: fee_asset_id.to_string(),
    });
    apply_required_fee(
        rpc,
        database_api_id,
        &mut transaction,
        fee_asset_id,
        max_fee,
        asset_issue_json,
    )?;
    sign_and_broadcast(
        rpc,
        network_broadcast_api_id,
        transaction,
        wif,
        signing_public_key,
        asset_issue_json,
        &format!("asset_issue {amount} {asset_id} to {recipient_id}"),
    )?;
    wait_for_balance_at_least(
        rpc,
        database_api_id,
        recipient_id,
        asset_id,
        before + amount,
    )?;
    Ok(())
}

fn create_unmatched_order(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    network_broadcast_api_id: u64,
    wif: &str,
    signing_public_key: &str,
    fee_asset_id: &str,
    max_fee: i64,
    seller_id: &str,
    sell_asset_id: &str,
    sell_amount: i64,
    receive_asset_id: &str,
    receive_amount: i64,
) -> Result<String, Box<dyn Error>> {
    let header = next_transaction_header(rpc, database_api_id, Duration::from_secs(60))?;
    let order_expiration =
        next_transaction_header(rpc, database_api_id, Duration::from_secs(3600))?.expiration;
    let mut transaction = build_limit_order_create_transaction(LimitOrderCreateTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        transaction_expiration: header.expiration,
        fee_amount: 0,
        fee_asset_id: fee_asset_id.to_string(),
        seller_id: seller_id.to_string(),
        amount_to_sell_amount: sell_amount,
        amount_to_sell_asset_id: sell_asset_id.to_string(),
        min_to_receive_amount: receive_amount,
        min_to_receive_asset_id: receive_asset_id.to_string(),
        order_expiration,
        fill_or_kill: false,
    });
    apply_required_fee(
        rpc,
        database_api_id,
        &mut transaction,
        fee_asset_id,
        max_fee,
        limit_order_create_json,
    )?;
    sign_and_broadcast(
        rpc,
        network_broadcast_api_id,
        transaction,
        wif,
        signing_public_key,
        limit_order_create_json,
        "limit_order_create unmatched test order",
    )?;
    wait_for_limit_order(
        rpc,
        database_api_id,
        seller_id,
        sell_asset_id,
        receive_asset_id,
    )
}

fn cancel_order(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    network_broadcast_api_id: u64,
    wif: &str,
    signing_public_key: &str,
    fee_asset_id: &str,
    max_fee: i64,
    seller_id: &str,
    order_id: &str,
) -> Result<(), Box<dyn Error>> {
    let header = next_transaction_header(rpc, database_api_id, Duration::from_secs(60))?;
    let mut transaction = build_limit_order_cancel_transaction(LimitOrderCancelTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        expiration: header.expiration,
        fee_amount: 0,
        fee_asset_id: fee_asset_id.to_string(),
        fee_paying_account_id: seller_id.to_string(),
        order_id: order_id.to_string(),
    });
    apply_required_fee(
        rpc,
        database_api_id,
        &mut transaction,
        fee_asset_id,
        max_fee,
        limit_order_cancel_json,
    )?;
    sign_and_broadcast(
        rpc,
        network_broadcast_api_id,
        transaction,
        wif,
        signing_public_key,
        limit_order_cancel_json,
        &format!("limit_order_cancel {order_id}"),
    )?;
    Ok(())
}

fn sign_and_broadcast<F, E>(
    rpc: &mut GrapheneRpc,
    network_broadcast_api_id: u64,
    transaction: Transaction,
    wif: &str,
    expected_public_key: &str,
    renderer: F,
    label: &str,
) -> Result<SignedTransaction, Box<dyn Error>>
where
    F: Fn(&SignedTransaction) -> Result<Value, E>,
    E: Error + 'static,
{
    println!("Broadcasting {label}");
    println!("Digest: {}", hex(&transaction.signature_digest_bytes()?));
    let signed_transaction = sign_and_broadcast_transaction(
        rpc,
        network_broadcast_api_id,
        &transaction,
        wif,
        expected_public_key,
        renderer,
    )?;
    println!("Broadcast submitted: {label}");
    Ok(signed_transaction)
}

fn env_i64(key: &str, default: i64) -> Result<i64, Box<dyn Error>> {
    env::var(key)
        .ok()
        .map(|value| value.parse::<i64>())
        .transpose()
        .map_err(Into::into)
        .map(|value| value.unwrap_or(default))
}

fn env_flag(key: &str) -> bool {
    matches!(
        env::var(key).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn timestamp_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn suffix_tail(value: &str) -> String {
    value
        .chars()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_tail_uses_last_eight_digits() {
        assert_eq!(suffix_tail("1234567890"), "34567890");
    }

    #[test]
    fn env_flag_accepts_truthy_values() {
        unsafe {
            env::set_var("SWAPLOCK_TEST_FLAG", "true");
        }
        assert!(env_flag("SWAPLOCK_TEST_FLAG"));
        unsafe {
            env::remove_var("SWAPLOCK_TEST_FLAG");
        }
    }
}
