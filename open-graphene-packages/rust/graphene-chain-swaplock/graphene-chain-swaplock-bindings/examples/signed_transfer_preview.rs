use std::env;
use std::error::Error;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::FcSerialize;
use graphene_chain_swaplock_bindings::generated::fc::{
    decode_public_key, verify_compact_signature_public_key,
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
    let amount = env::var("SWAPLOCK_TRANSFER_AMOUNT")
        .ok()
        .map(|value| value.parse::<i64>())
        .transpose()?
        .unwrap_or(1);

    let mut rpc = GrapheneRpc::connect(&rpc_url)?;
    let database_api_id = rpc.database_api_id()?;

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
            from: AccountId(from_id),
            to: AccountId(to_id),
            amount: Asset {
                amount,
                asset_id: AssetId(asset_id),
            },
            memo: None,
            extensions: FutureExtensions::VoidT(Box::new(())),
        }))],
        extensions: FutureExtensions::VoidT(Box::new(())),
    };

    let signed_transaction = transaction.signed_with_wif(&wif)?;
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
    rpc.call_network_broadcast(
        network_broadcast_api_id,
        "broadcast_transaction",
        json!([signed_transaction_json(&signed_transaction)?]),
    )?;
    println!("broadcast_result: broadcast_transaction submitted");

    Ok(())
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
}
