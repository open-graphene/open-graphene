use std::env;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use graphene_chain_swaplock_bindings::generated::FcSerialize;
use graphene_chain_swaplock_bindings::generated::fc::{
    decode_public_key, is_graphene_canonical_compact_signature, verify_compact_signature_public_key,
};
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction};
use graphene_chain_swaplock_bindings::sdk::asset_create::{
    AssetCreateTransactionInput, build_asset_create_transaction, signed_transaction_json,
};
use serde_json::{Value, json};
use tungstenite::{Message, WebSocket, connect};

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

    let dgp = rpc
        .call_database(database_api_id, "get_objects", json!([["2.1.0"]]))?
        .get(0)
        .cloned()
        .ok_or("dynamic global properties object was not returned")?;
    let head_block_number = dgp
        .get("head_block_number")
        .and_then(Value::as_u64)
        .ok_or("dynamic global properties missing head_block_number")?;
    let head_block_id = dgp
        .get("head_block_id")
        .and_then(Value::as_str)
        .ok_or("dynamic global properties missing head_block_id")?;
    let head_time = dgp
        .get("time")
        .and_then(Value::as_str)
        .ok_or("dynamic global properties missing time")?;

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

    let header = open_graphene_sdk_core::transaction_header_from_head(
        &open_graphene_sdk_core::HeadBlock {
            number: head_block_number,
            id: head_block_id.to_string(),
            time: head_time.to_string(),
        },
        Duration::from_secs(60),
    )?;

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

    let fee = required_asset_create_fee(&mut rpc, database_api_id, &transaction, &fee_asset_id)?;
    if fee.amount > max_fee {
        return Err(format!(
            "required fee {} exceeds SWAPLOCK_MAX_FEE {}; refusing to sign",
            fee.amount, max_fee
        )
        .into());
    }
    set_asset_create_fee(&mut transaction, fee.clone())?;
    let balance_before = account_balance(&mut rpc, database_api_id, &issuer_id, &fee.asset_id.0)?;
    if balance_before < fee.amount {
        return Err(format!(
            "insufficient balance for fee asset {}: balance {}, required {}",
            fee.asset_id.0, balance_before, fee.amount
        )
        .into());
    }

    let signed_transaction = transaction.signed_with_wif(&wif)?;
    if !is_graphene_canonical_compact_signature(&signed_transaction.signatures[0].0) {
        return Err("signature is not Graphene canonical; refusing to broadcast".into());
    }
    let matches_public_key = verify_compact_signature_public_key(
        transaction.signature_digest_bytes()?,
        &signed_transaction.signatures[0].0,
        decode_public_key(&issuer_signing_public_key, Some("BTS"))?,
    )?;
    if !matches_public_key {
        return Err("signature public key verification failed; refusing to broadcast".into());
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
    rpc.call_network_broadcast(
        network_broadcast_api_id,
        "broadcast_transaction",
        json!([signed_transaction_json(&signed_transaction)?]),
    )?;
    println!("Broadcast: submitted");

    let created_id = wait_for_asset(&mut rpc, database_api_id, &symbol)?
        .ok_or("broadcast submitted but new asset was not found")?;
    println!("Created asset: {symbol} ({created_id})");

    Ok(())
}

fn required_asset_create_fee(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    transaction: &graphene_chain_swaplock_bindings::generated::types::Transaction,
    asset_id: &str,
) -> Result<Asset, Box<dyn Error>> {
    let signed = SignedTransaction {
        ref_block_num: transaction.ref_block_num,
        ref_block_prefix: transaction.ref_block_prefix,
        expiration: transaction.expiration.clone(),
        operations: transaction.operations.clone(),
        extensions: transaction.extensions.clone(),
        signatures: vec![],
    };
    let tx_json = signed_transaction_json(&signed)?;
    let op_json = tx_json
        .get("operations")
        .and_then(Value::as_array)
        .and_then(|ops| ops.first())
        .cloned()
        .ok_or("asset-create JSON missing operation")?;
    let result = rpc.call_database(api_id, "get_required_fees", json!([[op_json], asset_id]))?;
    parse_asset(
        result
            .as_array()
            .and_then(|values| values.first())
            .ok_or("get_required_fees returned no fee")?,
    )
}

fn set_asset_create_fee(
    transaction: &mut graphene_chain_swaplock_bindings::generated::types::Transaction,
    fee: Asset,
) -> Result<(), Box<dyn Error>> {
    let Some(graphene_chain_swaplock_bindings::generated::static_variants::Operation::AssetCreateOperation(operation)) = transaction.operations.first_mut() else {
        return Err("transaction does not contain asset_create operation".into());
    };
    operation.fee = fee;
    Ok(())
}

fn lookup_account_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<String, Box<dyn Error>> {
    let result = rpc.call_database(api_id, "lookup_accounts", json!([account_name, 1]))?;
    let pair = result
        .as_array()
        .and_then(|values| values.first())
        .ok_or_else(|| format!("account not found: {account_name}"))?;
    let returned_name = pair
        .get(0)
        .and_then(Value::as_str)
        .ok_or("lookup_accounts result missing account name")?;
    if returned_name != account_name {
        return Err(format!("account not found: {account_name}").into());
    }
    pair.get(1)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "lookup_accounts result missing account id".into())
}

fn lookup_asset_id_optional(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    symbol: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    let result = rpc.call_database(api_id, "lookup_asset_symbols", json!([[symbol]]))?;
    let Some(first) = result.as_array().and_then(|values| values.first()) else {
        return Ok(None);
    };
    if first.is_null() {
        return Ok(None);
    }
    let returned_symbol = first
        .get("symbol")
        .and_then(Value::as_str)
        .ok_or("lookup_asset_symbols result missing symbol")?;
    if returned_symbol != symbol {
        return Ok(None);
    }
    Ok(first.get("id").and_then(Value::as_str).map(str::to_string))
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
        .get(0)
        .and_then(Value::as_str)
        .map(str::to_string))
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

fn parse_asset(value: &Value) -> Result<Asset, Box<dyn Error>> {
    Ok(Asset {
        amount: value
            .get("amount")
            .and_then(json_i64)
            .ok_or("asset missing integer amount")?,
        asset_id: graphene_chain_swaplock_bindings::generated::ids::AssetId(
            value
                .get("asset_id")
                .and_then(Value::as_str)
                .ok_or("asset missing asset_id")?
                .to_string(),
        ),
    })
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

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
