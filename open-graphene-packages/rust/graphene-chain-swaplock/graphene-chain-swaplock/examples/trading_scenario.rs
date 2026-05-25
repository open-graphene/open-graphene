use std::env;
use std::error::Error;
use std::thread::sleep;
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
use graphene_chain_swaplock::bindings::generated::fc::{
    decode_public_key, is_graphene_canonical_compact_signature, verify_compact_signature_public_key,
};
use graphene_chain_swaplock::bindings::generated::ids::AssetId;
use graphene_chain_swaplock::bindings::generated::static_variants::Operation;
use graphene_chain_swaplock::bindings::generated::types::{Asset, SignedTransaction, Transaction};
use graphene_chain_swaplock::limit_order_cancel::{
    build_limit_order_cancel_transaction, signed_transaction_json as limit_order_cancel_json,
    LimitOrderCancelTransactionInput,
};
use graphene_chain_swaplock::limit_order_create::{
    build_limit_order_create_transaction, signed_transaction_json as limit_order_create_json,
    LimitOrderCreateTransactionInput,
};
use graphene_chain_swaplock::transfer::{
    build_transfer_transaction, signed_transaction_json as transfer_json, TransferTransactionInput,
};
use serde_json::{json, Value};
use tungstenite::{connect, Message, WebSocket};

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

struct GrapheneRpc {
    socket: WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    next_id: u64,
}

impl GrapheneRpc {
    fn connect(url: &str) -> Result<Self, Box<dyn Error>> {
        let (socket, _) = connect(url)?;
        Ok(Self { socket, next_id: 1 })
    }

    fn call_raw(&mut self, params: Value) -> Result<Value, Box<dyn Error>> {
        let id = self.next_id;
        self.next_id += 1;
        self.socket.send(Message::Text(
            json!({"id": id, "method": "call", "params": params}).to_string(),
        ))?;
        loop {
            let response = self.socket.read()?;
            let Message::Text(text) = response else {
                continue;
            };
            let value: Value = serde_json::from_str(&text)?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(format!("Graphene RPC error: {error}").into());
            }
            return value
                .get("result")
                .cloned()
                .ok_or_else(|| "Graphene RPC response missing result".into());
        }
    }

    fn database_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        self.call_raw(json!([1, "database", []]))?
            .as_u64()
            .ok_or_else(|| "database API id is not an integer".into())
    }

    fn network_broadcast_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        self.call_raw(json!([1, "network_broadcast", []]))?
            .as_u64()
            .ok_or_else(|| "network_broadcast API id is not an integer".into())
    }

    fn call_database(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        self.call_raw(json!([api_id, method, params]))
    }

    fn call_network_broadcast(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        self.call_raw(json!([api_id, method, params]))
    }
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

    let header = next_header(rpc, database_api_id)?;
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
    set_required_fee(
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

    let header = next_header(rpc, database_api_id)?;
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
    set_required_fee(
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

    let header = next_header(rpc, database_api_id)?;
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
    set_required_fee(
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

    let header = next_header(rpc, database_api_id)?;
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
    set_required_fee(
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
    let header = next_header(rpc, database_api_id)?;
    let order_expiration = open_graphene_sdk_core::transaction_header_from_head(
        &head_block(rpc, database_api_id)?,
        Duration::from_secs(3600),
    )?
    .expiration;
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
    set_required_fee(
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
    let header = next_header(rpc, database_api_id)?;
    let mut transaction = build_limit_order_cancel_transaction(LimitOrderCancelTransactionInput {
        ref_block_num: header.ref_block_num,
        ref_block_prefix: header.ref_block_prefix,
        expiration: header.expiration,
        fee_amount: 0,
        fee_asset_id: fee_asset_id.to_string(),
        fee_paying_account_id: seller_id.to_string(),
        order_id: order_id.to_string(),
    });
    set_required_fee(
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

fn set_required_fee<F, E>(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    transaction: &mut Transaction,
    fee_asset_id: &str,
    max_fee: i64,
    renderer: F,
) -> Result<(), Box<dyn Error>>
where
    F: Fn(&SignedTransaction) -> Result<Value, E>,
    E: Error + 'static,
{
    let fee = required_fee(rpc, database_api_id, transaction, fee_asset_id, renderer)?;
    if fee.amount > max_fee {
        return Err(format!(
            "required fee {} exceeds SWAPLOCK_MAX_FEE {}; refusing to sign",
            fee.amount, max_fee
        )
        .into());
    }
    set_first_operation_fee(transaction, fee)
}

fn required_fee<F, E>(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
    transaction: &Transaction,
    fee_asset_id: &str,
    renderer: F,
) -> Result<Asset, Box<dyn Error>>
where
    F: Fn(&SignedTransaction) -> Result<Value, E>,
    E: Error + 'static,
{
    let unsigned = SignedTransaction {
        ref_block_num: transaction.ref_block_num,
        ref_block_prefix: transaction.ref_block_prefix,
        expiration: transaction.expiration.clone(),
        operations: transaction.operations.clone(),
        extensions: transaction.extensions.clone(),
        signatures: Vec::new(),
    };
    let tx_json = renderer(&unsigned).map_err(|err| -> Box<dyn Error> { Box::new(err) })?;
    let op_json = tx_json
        .get("operations")
        .and_then(Value::as_array)
        .and_then(|operations| operations.first())
        .cloned()
        .ok_or("transaction JSON missing first operation")?;
    let result = rpc.call_database(
        database_api_id,
        "get_required_fees",
        json!([[op_json], fee_asset_id]),
    )?;
    parse_asset(
        result
            .as_array()
            .and_then(|values| values.first())
            .ok_or("get_required_fees returned no fee")?,
    )
}

fn set_first_operation_fee(
    transaction: &mut Transaction,
    fee: Asset,
) -> Result<(), Box<dyn Error>> {
    let operation = transaction
        .operations
        .first_mut()
        .ok_or("transaction contains no operations")?;
    match operation {
        Operation::TransferOperation(operation) => operation.fee = fee,
        Operation::LimitOrderCreateOperation(operation) => operation.fee = fee,
        Operation::LimitOrderCancelOperation(operation) => operation.fee = fee,
        Operation::AccountCreateOperation(operation) => operation.fee = fee,
        Operation::AssetCreateOperation(operation) => operation.fee = fee,
        Operation::AssetIssueOperation(operation) => operation.fee = fee,
        _ => return Err("unsupported operation for fee injection".into()),
    }
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
    let signed_transaction = transaction.signed_with_wif(wif)?;
    let signature = signed_transaction
        .signatures
        .first()
        .ok_or("signed transaction has no signature")?;
    if !is_graphene_canonical_compact_signature(&signature.0) {
        return Err("signature is not Graphene canonical; refusing to broadcast".into());
    }
    let matches_public_key = verify_compact_signature_public_key(
        transaction.signature_digest_bytes()?,
        &signature.0,
        decode_public_key(expected_public_key, Some("BTS"))?,
    )?;
    if !matches_public_key {
        return Err("signature public key verification failed; refusing to broadcast".into());
    }

    println!("Broadcasting {label}");
    println!("Digest: {}", hex(&transaction.signature_digest_bytes()?));
    rpc.call_network_broadcast(
        network_broadcast_api_id,
        "broadcast_transaction",
        json!([renderer(&signed_transaction).map_err(|err| -> Box<dyn Error> { Box::new(err) })?]),
    )?;
    println!("Broadcast submitted: {label}");
    Ok(signed_transaction)
}

fn next_header(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
) -> Result<open_graphene_sdk_core::TransactionHeader, Box<dyn Error>> {
    open_graphene_sdk_core::transaction_header_from_head(
        &head_block(rpc, database_api_id)?,
        Duration::from_secs(60),
    )
    .map_err(Into::into)
}

fn head_block(
    rpc: &mut GrapheneRpc,
    database_api_id: u64,
) -> Result<open_graphene_sdk_core::HeadBlock, Box<dyn Error>> {
    let dgp = rpc
        .call_database(database_api_id, "get_objects", json!([["2.1.0"]]))?
        .get(0)
        .cloned()
        .ok_or("dynamic global properties object was not returned")?;
    Ok(open_graphene_sdk_core::HeadBlock {
        number: dgp
            .get("head_block_number")
            .and_then(Value::as_u64)
            .ok_or("dynamic global properties missing head_block_number")?,
        id: dgp
            .get("head_block_id")
            .and_then(Value::as_str)
            .ok_or("dynamic global properties missing head_block_id")?
            .to_string(),
        time: dgp
            .get("time")
            .and_then(Value::as_str)
            .ok_or("dynamic global properties missing time")?
            .to_string(),
    })
}

fn lookup_account_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<String, Box<dyn Error>> {
    lookup_account_id_optional(rpc, api_id, account_name)?
        .ok_or_else(|| format!("account not found: {account_name}").into())
}

fn lookup_account_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let result = rpc.call_database(api_id, "lookup_accounts", json!([account_name, 1]))?;
    let Some(pair) = result.as_array().and_then(|values| values.first()) else {
        return Ok(None);
    };
    let returned_name = pair
        .get(0)
        .and_then(Value::as_str)
        .ok_or("lookup_accounts result missing account name")?;
    if returned_name != account_name {
        return Ok(None);
    }
    pair.get(1)
        .and_then(Value::as_str)
        .map(|id| Some(id.to_string()))
        .ok_or_else(|| "lookup_accounts result missing account id".into())
}

fn wait_for_account(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    for _ in 0..20 {
        if let Some(account_id) = lookup_account_id_optional(rpc, api_id, account_name)? {
            return Ok(Some(account_id));
        }
        sleep(Duration::from_millis(500));
    }
    Ok(None)
}

fn lookup_asset_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let result = rpc.call_database(api_id, "lookup_asset_symbols", json!([[symbol]]))?;
    let Some(asset) = result.as_array().and_then(|values| values.first()) else {
        return Ok(None);
    };
    if asset.is_null() {
        return Ok(None);
    }
    let returned_symbol = asset
        .get("symbol")
        .and_then(Value::as_str)
        .ok_or("lookup_asset_symbols result missing symbol")?;
    if returned_symbol != symbol {
        return Ok(None);
    }
    asset
        .get("id")
        .and_then(Value::as_str)
        .map(|id| Some(id.to_string()))
        .ok_or_else(|| "lookup_asset_symbols result missing id".into())
}

fn wait_for_asset(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    for _ in 0..20 {
        if let Some(asset_id) = lookup_asset_id_optional(rpc, api_id, symbol)? {
            return Ok(Some(asset_id));
        }
        sleep(Duration::from_millis(500));
    }
    Ok(None)
}

fn active_public_key_for_account(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let account = rpc
        .call_database(api_id, "get_objects", json!([[account_id]]))?
        .get(0)
        .cloned()
        .ok_or("account object was not returned")?;
    let Some(key_auths) = account
        .get("active")
        .and_then(|active| active.get("key_auths"))
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    if key_auths.len() != 1 {
        return Ok(None);
    }
    Ok(key_auths[0]
        .as_array()
        .and_then(|pair| pair.first())
        .and_then(Value::as_str)
        .map(ToOwned::to_owned))
}

fn account_balance(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
    asset_id: &str,
) -> Result<i64, Box<dyn Error>> {
    let balances = rpc.call_database(
        api_id,
        "get_account_balances",
        json!([account_id, [asset_id]]),
    )?;
    let balance = balances
        .as_array()
        .and_then(|values| values.first())
        .ok_or("get_account_balances returned no balance")?;
    if balance.get("asset_id").and_then(Value::as_str) != Some(asset_id) {
        return Err("get_account_balances returned unexpected asset_id".into());
    }
    balance
        .get("amount")
        .and_then(json_i64)
        .ok_or_else(|| "balance missing integer amount".into())
}

fn wait_for_balance_at_least(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
    asset_id: &str,
    minimum: i64,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..20 {
        let balance = account_balance(rpc, api_id, account_id, asset_id)?;
        if balance >= minimum {
            println!("Balance ok: {account_id} has {balance} {asset_id}");
            return Ok(());
        }
        sleep(Duration::from_millis(500));
    }
    Err(format!("balance for {account_id} {asset_id} did not reach {minimum}").into())
}

fn wait_for_limit_order(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    seller_id: &str,
    asset_a_id: &str,
    asset_b_id: &str,
) -> Result<String, Box<dyn Error>> {
    for _ in 0..20 {
        if let Some(order_id) = find_limit_order(rpc, api_id, seller_id, asset_a_id, asset_b_id)? {
            return Ok(order_id);
        }
        if let Some(order_id) = find_limit_order(rpc, api_id, seller_id, asset_b_id, asset_a_id)? {
            return Ok(order_id);
        }
        sleep(Duration::from_millis(500));
    }
    Err(format!("limit order for seller {seller_id} was not found").into())
}

fn find_limit_order(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    seller_id: &str,
    base_asset_id: &str,
    quote_asset_id: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let orders = rpc.call_database(
        api_id,
        "get_limit_orders",
        json!([base_asset_id, quote_asset_id, 100]),
    )?;
    let Some(orders) = orders.as_array() else {
        return Err("get_limit_orders result is not an array".into());
    };
    for order in orders {
        if order.get("seller").and_then(Value::as_str) == Some(seller_id) {
            if let Some(id) = order.get("id").and_then(Value::as_str) {
                return Ok(Some(id.to_string()));
            }
        }
    }
    Ok(None)
}

fn wait_for_order_gone(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    order_id: &str,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..20 {
        let result = rpc.call_database(api_id, "get_objects", json!([[order_id]]))?;
        if result
            .as_array()
            .and_then(|values| values.first())
            .is_some_and(Value::is_null)
        {
            println!("Order canceled: {order_id}");
            return Ok(());
        }
        sleep(Duration::from_millis(500));
    }
    Err(format!("order still exists after cancel: {order_id}").into())
}

fn parse_asset(value: &Value) -> Result<Asset, Box<dyn Error>> {
    Ok(Asset {
        amount: value
            .get("amount")
            .and_then(json_i64)
            .ok_or("asset missing integer amount")?,
        asset_id: AssetId(
            value
                .get("asset_id")
                .and_then(Value::as_str)
                .ok_or("asset missing asset_id")?
                .to_string(),
        ),
    })
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
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
