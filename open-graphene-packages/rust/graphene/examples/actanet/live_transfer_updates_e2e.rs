#[path = "common.rs"]
mod common;

use std::env;
use std::time::Duration;

use graphene::Graphene;

const WAIT_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let wif = env::var("ACTANET_ACTIVE_WIF")?;
    let from_account = env::var("ACTANET_ACCOUNT")?;
    let to_account = env::var("ACTANET_TO_ACCOUNT").unwrap_or_else(|_| "1.2.0".to_string());

    println!(
        "this example broadcasts a real transfer on the configured Actanet testnet; never use production funds"
    );

    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let prepared = actanet
        .operations()
        .transfer()
        .from(&from_account)
        .to(&to_account)
        .amount_raw(1, "ACTA")
        .fee_asset("ACTA")
        .max_fee_raw(1_000_000)
        .prepare()
        .await?;
    let signed = actanet
        .operations()
        .sign_transfer_with_wif(prepared, &wif)
        .await?;

    let from_id = signed.from_id().to_string();
    let asset_id = signed.asset_id().to_string();
    println!(
        "prepared signed transfer: from {} to {} amount {} {} head block {} signatures {}",
        from_id,
        signed.to_id(),
        signed.amount(),
        asset_id,
        signed.head_block_number(),
        signed.signed_transaction().signatures.len()
    );

    let live = actanet.into_live()?;
    println!(
        "entered typed Actanet live mode on chain {}",
        live.chain_id()
    );

    let mut dgp = live
        .database()
        .subscribe_dynamic_global_properties_timeout(WAIT_TIMEOUT)
        .await?;
    println!(
        "initial DGP head block: {} {}",
        dgp.initial().head_block_number,
        dgp.initial().time
    );

    let mut balances = live
        .database()
        .account_balances_by_id(&from_id, [&asset_id])
        .subscribe_timeout(WAIT_TIMEOUT)
        .await?;
    let initial_balance = balances
        .initial()
        .iter()
        .find(|balance| balance.asset_type.0 == asset_id)
        .map(|balance| balance.balance);
    println!(
        "initial balance for {} {}: {:?}",
        from_id, asset_id, initial_balance
    );

    let mut history = live
        .history()?
        .account_history_by_id(&from_id)
        .limit(5)
        .offset(0)
        .subscribe_timeout(WAIT_TIMEOUT)
        .await?;
    println!(
        "initial account history entries: {}",
        history.initial().items().len()
    );

    let pending = live
        .network_broadcast()?
        .send_signed_transfer_with_callback(signed)
        .await?;
    println!("sent transfer through typed live network_broadcast wrapper");

    let confirmation = pending.wait_timeout(WAIT_TIMEOUT).await?;
    println!(
        "confirmed transfer: block {} trx_num {} id {}",
        confirmation.block_num(),
        confirmation.trx_num(),
        confirmation.id()
    );

    let dgp_update = dgp.next_update_timeout(WAIT_TIMEOUT).await?;
    println!(
        "DGP update after transfer: head block {} {}",
        dgp_update.head_block_number, dgp_update.time
    );

    let balance_updates = balances.next_update_timeout(WAIT_TIMEOUT).await?;
    println!("balance updates after transfer: {}", balance_updates.len());
    for balance in &balance_updates {
        println!(
            "balance update: {} owner {} asset {} amount {}",
            balance.id.0, balance.owner.0, balance.asset_type.0, balance.balance
        );
    }

    let history_updates = history.next_update_timeout(WAIT_TIMEOUT).await?;
    println!("history updates after transfer: {}", history_updates.len());
    for item in &history_updates {
        println!(
            "history update: {} block {} time {}",
            item.id.0, item.block_num, item.block_time
        );
    }

    Ok(())
}
