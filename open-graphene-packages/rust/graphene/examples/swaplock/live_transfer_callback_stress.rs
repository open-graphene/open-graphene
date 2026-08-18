use std::env;
use std::time::Duration;

use graphene::Graphene;

const DEFAULT_TRANSFER_COUNT: usize = 3;
const MAX_TRANSFER_COUNT: usize = 20;
const WAIT_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let wif = env::var("SWAPLOCK_ACTIVE_WIF")?;
    let from_account = env::var("SWAPLOCK_ACCOUNT")?;
    let to_account = env::var("SWAPLOCK_TO_ACCOUNT").unwrap_or_else(|_| "1.2.0".to_string());
    let transfer_count = env_transfer_count()?;

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("f7dc1f352cb6b8aef3d14a4aab8cb5592e440c79f8634252e327b40610b7784d")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let mut signed_transfers = Vec::with_capacity(transfer_count);
    println!("preparing and signing {transfer_count} transfers before entering live mode");
    for index in 0..transfer_count {
        let amount = (index + 1) as i64;
        let prepared = swaplock
            .operations()
            .transfer()
            .from(&from_account)
            .to(&to_account)
            .amount_raw(amount, "BTS")
            .fee_asset("BTS")
            .max_fee_raw(1_000_000)
            .prepare()
            .await?;
        let signed = swaplock
            .operations()
            .sign_transfer_with_wif(prepared, &wif)
            .await?;
        println!(
            "prepared #{}: amount={} {}, head block={}, signatures={}",
            index + 1,
            signed.amount(),
            signed.asset_id(),
            signed.head_block_number(),
            signed.signed_transaction().signatures.len()
        );
        signed_transfers.push(signed);
    }

    let live = swaplock.into_live()?;
    println!(
        "entered typed Swaplock live mode on chain {}",
        live.chain_id()
    );

    let mut dgp = live
        .database()
        .subscribe_dynamic_global_properties_timeout(WAIT_TIMEOUT)
        .await?;
    println!(
        "live dynamic_global_properties subscription initial head block {} {}",
        dgp.initial().head_block_number,
        dgp.initial().time
    );

    println!(
        "sending {} signed transfers through typed live network_broadcast wrapper",
        signed_transfers.len()
    );
    let mut pending = Vec::with_capacity(signed_transfers.len());
    for (index, signed) in signed_transfers.into_iter().enumerate() {
        let amount = signed.amount();
        let asset_id = signed.asset_id().to_string();
        let pending_confirmation = live
            .network_broadcast()?
            .send_signed_transfer_with_callback(signed)
            .await?;
        println!(
            "sent live #{}: amount={} {}, callback request is now in-flight",
            index + 1,
            amount,
            asset_id
        );
        pending.push((index + 1, amount, asset_id, pending_confirmation));
    }

    for (index, amount, asset_id, pending_confirmation) in pending {
        let confirmation = pending_confirmation.wait_timeout(WAIT_TIMEOUT).await?;
        println!(
            "confirmed live #{}: amount={} {}, block={}, trx_num={}, id={}",
            index,
            amount,
            asset_id,
            confirmation.block_num(),
            confirmation.trx_num(),
            confirmation.id()
        );
    }

    let update = dgp.next_update_timeout(WAIT_TIMEOUT).await?;
    println!(
        "live dynamic_global_properties update after transfers: head block {} {}",
        update.head_block_number, update.time
    );

    Ok(())
}

fn env_transfer_count() -> Result<usize, Box<dyn std::error::Error>> {
    let count = match env::var("SWAPLOCK_STRESS_TRANSFERS") {
        Ok(value) => value.parse::<usize>()?,
        Err(env::VarError::NotPresent) => DEFAULT_TRANSFER_COUNT,
        Err(error) => return Err(Box::new(error)),
    };

    if count == 0 || count > MAX_TRANSFER_COUNT {
        return Err(format!(
            "SWAPLOCK_STRESS_TRANSFERS must be in 1..={MAX_TRANSFER_COUNT}, got {count}"
        )
        .into());
    }

    Ok(count)
}
