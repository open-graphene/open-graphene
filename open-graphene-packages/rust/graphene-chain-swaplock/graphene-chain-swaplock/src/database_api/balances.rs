use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::Asset;
use open_graphene_sdk_core::{AccountIdRef, AssetIdRef};

use crate::rpc::GrapheneRpc;

pub fn account_balance(
    rpc: &mut GrapheneRpc,
    api_id: u64,
    account_id: &str,
    asset_id: &str,
) -> Result<i64, Box<dyn Error>> {
    ensure_database_api_id(rpc, api_id)?;
    let account_id = AccountIdRef::parse(account_id)?;
    let expected_asset_id = AssetIdRef::parse(asset_id)?;
    let balances: Vec<Asset> = serde_json::from_value(rpc.get_account_balances(
        api_id,
        account_id.to_string(),
        [expected_asset_id.to_string()],
    )?)?;
    let balance = balances
        .first()
        .ok_or_else(|| format!("missing balance for {account_id} {expected_asset_id}"))?;
    balance_amount_for_asset(balance, &expected_asset_id)
}

pub fn wait_for_balance_at_least(
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

fn balance_amount_for_asset(
    balance: &Asset,
    expected_asset_id: &AssetIdRef,
) -> Result<i64, Box<dyn Error>> {
    let returned_asset_id = AssetIdRef::parse(&balance.asset_id.0)?;
    if &returned_asset_id != expected_asset_id {
        return Err(format!(
            "unexpected balance asset id: expected {expected_asset_id}, got {returned_asset_id}"
        )
        .into());
    }
    Ok(balance.amount)
}

fn ensure_database_api_id(rpc: &mut GrapheneRpc, api_id: u64) -> Result<(), Box<dyn Error>> {
    let expected = rpc.database_api_id()?;
    if expected != api_id {
        return Err(format!("database API id mismatch: expected {expected}, got {api_id}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use graphene_chain_swaplock_bindings::generated::AssetId;

    use super::*;

    #[test]
    fn generated_asset_balance_accepts_decimal_string_amount() {
        let json = serde_json::json!({
            "amount": "12345",
            "asset_id": "1.3.0"
        });

        let balance: Asset = serde_json::from_value(json).expect("deserialize balance asset");
        let expected_asset_id = AssetIdRef::parse("1.3.0").unwrap();

        assert_eq!(
            balance_amount_for_asset(&balance, &expected_asset_id).unwrap(),
            12_345
        );
    }

    #[test]
    fn generated_asset_balance_rejects_unexpected_asset() {
        let balance = Asset::new(123, AssetId("1.3.1".to_string()));
        let expected_asset_id = AssetIdRef::parse("1.3.0").unwrap();

        let error = balance_amount_for_asset(&balance, &expected_asset_id).unwrap_err();

        assert!(error
            .to_string()
            .contains("unexpected balance asset id: expected 1.3.0, got 1.3.1"));
    }
}
