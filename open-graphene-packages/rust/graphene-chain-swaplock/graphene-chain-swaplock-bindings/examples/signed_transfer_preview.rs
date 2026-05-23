use std::env;
use std::error::Error;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::FcSerialize;
use graphene_chain_swaplock_bindings::generated::fc::{
    decode_public_key, is_graphene_canonical_compact_signature, verify_compact_signature_public_key,
};
use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_swaplock_bindings::generated::operations::TransferOperation;
use graphene_chain_swaplock_bindings::generated::static_variants::{FutureExtensions, Operation};
use graphene_chain_swaplock_bindings::generated::types::{Asset, SignedTransaction, Transaction};
use serde_json::{Value, json};
use time::PrimitiveDateTime;
use time::format_description;
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
            json!({
                "id": id,
                "method": "call",
                "params": params,
            })
            .to_string(),
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

    fn history_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        self.call_raw(json!([1, "history", []]))?
            .as_u64()
            .ok_or_else(|| "history API id is not an integer".into())
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

    fn call_history(
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

    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;
    let asset_precision = asset_precision(&mut rpc, database_api_id, &asset_id)?;
    let amount = transfer_amount_raw(&human_amount, asset_precision)?;

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

    let from_id = lookup_account_id(&mut rpc, database_api_id, &from_account)?;
    let to_id = lookup_account_id(&mut rpc, database_api_id, &to_account)?;
    let expected_public_key = match env::var("SWAPLOCK_ACTIVE_PUBLIC_KEY") {
        Ok(public_key) => Some(("env", public_key)),
        Err(env::VarError::NotPresent) => {
            active_public_key_for_account(&mut rpc, database_api_id, &from_id)?
                .map(|public_key| ("chain_active_authority", public_key))
        }
        Err(err) => return Err(err.into()),
    };
    let expiration = expiration_from_head_time(head_time, Duration::from_secs(60))?;
    let ref_block_num = (head_block_number & 0xffff) as u16;
    let ref_block_prefix = ref_block_prefix_from_block_id(head_block_id)?;

    let fee = required_transfer_fee(
        &mut rpc,
        database_api_id,
        &from_id,
        &to_id,
        amount,
        &asset_id,
    )?;
    let expected_fee_amount = fee.amount;
    if fee.amount > max_fee {
        return Err(format!(
            "required fee {} exceeds SWAPLOCK_MAX_FEE {}; refusing to sign",
            fee.amount, max_fee
        )
        .into());
    }

    let transaction = Transaction {
        ref_block_num,
        ref_block_prefix,
        expiration,
        operations: vec![Operation::TransferOperation(Box::new(TransferOperation {
            fee,
            from: AccountId(from_id.clone()),
            to: AccountId(to_id.clone()),
            amount: Asset {
                amount,
                asset_id: AssetId(asset_id.clone()),
            },
            memo: None,
            extensions: FutureExtensions::VoidT(Box::new(())),
        }))],
        extensions: FutureExtensions::VoidT(Box::new(())),
    };

    let signed_transaction = transaction.signed_with_wif(&wif)?;
    if !is_graphene_canonical_compact_signature(&signed_transaction.signatures[0].0) {
        return Err("signature is not Graphene canonical; refusing to broadcast".into());
    }
    let signature_public_key_match = expected_public_key
        .as_ref()
        .map(|(source, public_key)| {
            verify_compact_signature_public_key(
                transaction.signature_digest_bytes()?,
                &signed_transaction.signatures[0].0,
                decode_public_key(public_key, Some("BTS"))?,
            )
            .map(|matches| (*source, matches))
        })
        .transpose()?;

    let digest_hex = hex(&transaction.signature_digest_bytes()?);
    ensure_signature_public_key_match(signature_public_key_match.map(|(_, matches)| matches))?;

    println!("read_only: false");
    println!("broadcast: true");
    println!("from_account: {from_account}");
    println!("from_id: {}", account_id_string(&transaction.operations[0]));
    println!("to_account: {to_account}");
    println!("asset_id: {asset_id}");
    println!("asset_precision: {asset_precision}");
    println!("amount: {human_amount}");
    println!("amount_raw: {amount}");
    println!("head_block_number: {head_block_number}");
    println!("ref_block_num: {ref_block_num}");
    println!("ref_block_prefix: {ref_block_prefix}");
    if let Some((source, matches_expected_public_key)) = signature_public_key_match {
        println!("signature_public_key_source: {source}");
        println!("signature_public_key_matches: {matches_expected_public_key}");
    } else {
        println!("signature_public_key_matches: <skipped; no single active public key found>");
    }
    println!("transaction_hex: {}", hex(&transaction.to_fc_bytes()?));
    println!("digest_hex: {digest_hex}");
    if env_flag("SWAPLOCK_PRINT_SIGNED_TX") {
        println!(
            "signed_transaction_hex: {}",
            hex(&signed_transaction.to_fc_bytes()?)
        );
    } else {
        println!("signed_transaction_hex: <hidden; set SWAPLOCK_PRINT_SIGNED_TX=1 to print>");
    }

    let network_broadcast_api_id = rpc.network_broadcast_api_id()?;
    let history_api_id = rpc.history_api_id()?;
    rpc.call_network_broadcast(
        network_broadcast_api_id,
        "broadcast_transaction",
        json!([signed_transaction_json(&signed_transaction)?]),
    )?;
    println!("broadcast_result: broadcast_transaction submitted");

    let confirmation = wait_for_transfer_confirmation(
        &mut rpc,
        history_api_id,
        &from_id,
        &TransferConfirmationCriteria {
            from_id: &from_id,
            to_id: &to_id,
            amount,
            asset_id: &asset_id,
            fee_amount: expected_fee_amount,
            min_block_num: head_block_number,
        },
    )?
    .ok_or("broadcast submitted but transfer was not found in account history")?;
    println!("confirmation: found");
    println!("confirmation_id: {}", confirmation.id);
    println!("confirmation_block_num: {}", confirmation.block_num);
    println!("confirmation_trx_in_block: {}", confirmation.trx_in_block);
    println!("confirmation_op_in_trx: {}", confirmation.op_in_trx);
    println!("confirmation_virtual_op: {}", confirmation.virtual_op);

    Ok(())
}

struct TransferConfirmationCriteria<'a> {
    from_id: &'a str,
    to_id: &'a str,
    amount: i64,
    asset_id: &'a str,
    fee_amount: i64,
    min_block_num: u64,
}

struct TransferConfirmation {
    id: String,
    block_num: u64,
    trx_in_block: u64,
    op_in_trx: u64,
    virtual_op: u64,
}

fn wait_for_transfer_confirmation(
    rpc: &mut GrapheneRpc,
    history_api_id: u64,
    account_id: &str,
    criteria: &TransferConfirmationCriteria<'_>,
) -> Result<Option<TransferConfirmation>, Box<dyn Error>> {
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

    for attempt in 0..attempts {
        let history = rpc.call_history(
            history_api_id,
            "get_account_history",
            json!([account_id, "1.11.0", 20, "1.11.0"]),
        )?;
        if let Some(confirmation) = find_transfer_confirmation(&history, criteria)? {
            return Ok(Some(confirmation));
        }
        if attempt + 1 < attempts {
            std::thread::sleep(delay);
        }
    }

    Ok(None)
}

fn find_transfer_confirmation(
    history: &Value,
    criteria: &TransferConfirmationCriteria<'_>,
) -> Result<Option<TransferConfirmation>, Box<dyn Error>> {
    let Some(entries) = history.as_array() else {
        return Err("get_account_history result is not an array".into());
    };
    for entry in entries {
        if history_entry_matches_transfer(entry, criteria) {
            return Ok(Some(TransferConfirmation {
                id: entry
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or("history entry missing id")?
                    .to_string(),
                block_num: entry
                    .get("block_num")
                    .and_then(Value::as_u64)
                    .ok_or("history entry missing block_num")?,
                trx_in_block: entry
                    .get("trx_in_block")
                    .and_then(Value::as_u64)
                    .ok_or("history entry missing trx_in_block")?,
                op_in_trx: entry
                    .get("op_in_trx")
                    .and_then(Value::as_u64)
                    .ok_or("history entry missing op_in_trx")?,
                virtual_op: entry
                    .get("virtual_op")
                    .and_then(Value::as_u64)
                    .ok_or("history entry missing virtual_op")?,
            }));
        }
    }
    Ok(None)
}

fn history_entry_matches_transfer(
    entry: &Value,
    criteria: &TransferConfirmationCriteria<'_>,
) -> bool {
    if entry.get("block_num").and_then(Value::as_u64) < Some(criteria.min_block_num) {
        return false;
    }
    let Some(operation) = entry.get("op").and_then(Value::as_array) else {
        return false;
    };
    if operation.first().and_then(Value::as_u64) != Some(0) {
        return false;
    }
    let Some(payload) = operation.get(1) else {
        return false;
    };
    payload.get("from").and_then(Value::as_str) == Some(criteria.from_id)
        && payload.get("to").and_then(Value::as_str) == Some(criteria.to_id)
        && asset_value_matches(payload.get("amount"), criteria.amount, criteria.asset_id)
        && asset_value_matches(payload.get("fee"), criteria.fee_amount, criteria.asset_id)
}

fn asset_value_matches(value: Option<&Value>, amount: i64, asset_id: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    value.get("amount").and_then(json_i64) == Some(amount)
        && value.get("asset_id").and_then(Value::as_str) == Some(asset_id)
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

fn signed_transaction_json(
    signed_transaction: &SignedTransaction,
) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "ref_block_num": signed_transaction.ref_block_num,
        "ref_block_prefix": signed_transaction.ref_block_prefix,
        "expiration": signed_transaction.expiration,
        "operations": signed_transaction
            .operations
            .iter()
            .map(operation_json)
            .collect::<Result<Vec<_>, _>>()?,
        "extensions": [],
        "signatures": signed_transaction
            .signatures
            .iter()
            .map(|signature| hex(&signature.0))
            .collect::<Vec<_>>(),
    }))
}

fn operation_json(operation: &Operation) -> Result<Value, Box<dyn Error>> {
    match operation {
        Operation::TransferOperation(operation) => Ok(json!([
            0,
            {
                "fee": asset_json(&operation.fee),
                "from": operation.from.0,
                "to": operation.to.0,
                "amount": asset_json(&operation.amount),
                "memo": null,
                "extensions": []
            }
        ])),
        _ => Err("signed transfer preview can only broadcast transfer operations".into()),
    }
}

fn asset_json(asset: &Asset) -> Value {
    json!({
        "amount": asset.amount,
        "asset_id": asset.asset_id.0,
    })
}

fn lookup_account_id(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_name: &str,
) -> Result<String, Box<dyn Error>> {
    let result = rpc.call_database(api_id, "lookup_accounts", json!([account_name, 1]))?;
    let Some(pair) = result.as_array().and_then(|values| values.first()) else {
        return Err(format!("account not found: {account_name}").into());
    };
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

fn asset_precision(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    asset_id: &str,
) -> Result<u8, Box<dyn Error>> {
    let asset = rpc
        .call_database(api_id, "get_objects", json!([[asset_id]]))?
        .get(0)
        .cloned()
        .ok_or("asset object was not returned")?;
    let precision = asset
        .get("precision")
        .and_then(Value::as_u64)
        .ok_or("asset object missing precision")?;
    u8::try_from(precision).map_err(|_| "asset precision out of u8 range".into())
}

fn transfer_amount_raw(human_amount: &str, precision: u8) -> Result<i64, Box<dyn Error>> {
    if let Ok(raw_amount) = env::var("SWAPLOCK_TRANSFER_RAW_AMOUNT") {
        return Ok(raw_amount.parse()?);
    }
    decimal_to_raw_amount(human_amount, precision)
}

fn decimal_to_raw_amount(value: &str, precision: u8) -> Result<i64, Box<dyn Error>> {
    let value = value.trim();
    if value.is_empty() {
        return Err("amount must not be empty".into());
    }
    if value.starts_with('-') {
        return Err("amount must not be negative".into());
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() > 2
        || parts
            .iter()
            .any(|part| !part.chars().all(|c| c.is_ascii_digit()))
    {
        return Err("amount must be a decimal number".into());
    }
    let integer = if parts[0].is_empty() { "0" } else { parts[0] };
    let mut fractional = parts.get(1).copied().unwrap_or("").to_string();
    while fractional.ends_with('0') {
        fractional.pop();
    }
    if fractional.len() > precision as usize {
        return Err(format!("amount supports at most {precision} decimal places").into());
    }
    while fractional.len() < precision as usize {
        fractional.push('0');
    }
    let raw = format!("{integer}{fractional}");
    let raw = raw.trim_start_matches('0');
    Ok(if raw.is_empty() { 0 } else { raw.parse()? })
}

fn required_transfer_fee(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    from_id: &str,
    to_id: &str,
    amount: i64,
    asset_id: &str,
) -> Result<Asset, Box<dyn Error>> {
    let transfer_json = json!([
        0,
        {
            "fee": { "amount": 0, "asset_id": asset_id },
            "from": from_id,
            "to": to_id,
            "amount": { "amount": amount, "asset_id": asset_id },
            "memo": null,
            "extensions": []
        }
    ]);
    let result = rpc.call_database(
        api_id,
        "get_required_fees",
        json!([[transfer_json], asset_id]),
    )?;
    let fee = result
        .as_array()
        .and_then(|values| values.first())
        .ok_or("get_required_fees returned no fee")?;
    let amount = fee
        .get("amount")
        .and_then(json_i64)
        .ok_or("fee missing integer amount")?;
    let asset_id = fee
        .get("asset_id")
        .and_then(Value::as_str)
        .ok_or("fee missing asset_id")?
        .to_string();

    Ok(Asset {
        amount,
        asset_id: AssetId(asset_id),
    })
}

fn ref_block_prefix_from_block_id(block_id: &str) -> Result<u32, Box<dyn Error>> {
    let bytes = decode_hex(block_id)?;
    if bytes.len() < 8 {
        return Err("block id must contain at least 8 bytes".into());
    }
    Ok(u32::from_le_bytes(bytes[4..8].try_into()?))
}

fn expiration_from_head_time(head_time: &str, offset: Duration) -> Result<String, Box<dyn Error>> {
    let format = format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]")?;
    let parsed = PrimitiveDateTime::parse(head_time, &format)?;
    let expiration = parsed + offset;
    Ok(expiration.format(&format)?)
}

fn decode_hex(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !value.len().is_multiple_of(2) {
        return Err("hex value has odd length".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).map_err(Into::into))
        .collect()
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn account_id_string(operation: &Operation) -> &str {
    match operation {
        Operation::TransferOperation(operation) => &operation.from.0,
        _ => "",
    }
}

#[cfg(test)]
mod tests {
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
    fn decimal_amount_converts_to_raw_amount_using_asset_precision() {
        assert_eq!(decimal_to_raw_amount("1", 5).unwrap(), 100_000);
        assert_eq!(decimal_to_raw_amount("1.23", 5).unwrap(), 123_000);
        assert_eq!(decimal_to_raw_amount("0.00001", 5).unwrap(), 1);
        assert_eq!(decimal_to_raw_amount("1.23000", 5).unwrap(), 123_000);
        assert_eq!(
            decimal_to_raw_amount("0.000001", 5)
                .expect_err("too many decimals fail")
                .to_string(),
            "amount supports at most 5 decimal places"
        );
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
        let confirmation = find_transfer_confirmation(
            &history,
            &TransferConfirmationCriteria {
                from_id: "1.2.100",
                to_id: "1.2.0",
                amount: 100000,
                asset_id: "1.3.0",
                fee_amount: 10,
                min_block_num: 123,
            },
        )
        .unwrap()
        .expect("matching transfer is confirmed");

        assert_eq!(confirmation.id, "1.11.22");
        assert_eq!(confirmation.block_num, 123);
        assert_eq!(confirmation.trx_in_block, 1);
        assert_eq!(confirmation.op_in_trx, 0);
        assert_eq!(confirmation.virtual_op, 0);
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
            find_transfer_confirmation(
                &history,
                &TransferConfirmationCriteria {
                    from_id: "1.2.100",
                    to_id: "1.2.0",
                    amount: 100000,
                    asset_id: "1.3.0",
                    fee_amount: 10,
                    min_block_num: 123,
                },
            )
            .unwrap()
            .is_none()
        );
    }
}
