use std::env;
use std::time::Duration;

use graphene::Graphene;
use graphene_chain_swaplock_api::PendingBroadcastConfirmation;

const DEFAULT_TRANSFER_COUNT: usize = 3;
const MAX_TRANSFER_COUNT: usize = 20;
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(30);

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
        .chain_id("f2491c85896bb49f936152d59b850ca05bb8e09d9027eb630267697ee483b05e")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let initial_head_block = {
        let subscription = swaplock
            .database()
            .dynamic_global_properties()
            .subscribe()
            .await?;
        let initial = subscription.initial().clone();
        println!(
            "registered dynamic_global_properties subscription at head block {} {}",
            initial.head_block_number, initial.time
        );
        println!(
            "dropping the subscription handle but keeping the server callback active on this websocket"
        );
        initial.head_block_number
    };

    println!("preparing and signing {transfer_count} transfers before broadcasting");
    let mut signed_transfers = Vec::with_capacity(transfer_count);
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

    println!(
        "sending {} broadcast_transaction_with_callback requests without waiting between sends",
        signed_transfers.len()
    );
    let mut pending = Vec::with_capacity(signed_transfers.len());
    for (index, signed) in signed_transfers.into_iter().enumerate() {
        let amount = signed.amount();
        let asset_id = signed.asset_id().to_string();
        let pending_confirmation = swaplock
            .network_broadcast()
            .send_signed_transfer_with_callback(signed)
            .await?;
        println!(
            "sent #{}: amount={} {}, callback request is now in-flight",
            index + 1,
            amount,
            asset_id
        );
        pending.push(PendingTransfer {
            index: index + 1,
            amount,
            asset_id,
            pending: pending_confirmation,
        });
    }

    println!("collecting confirmations after all broadcasts have been sent");
    for transfer in pending {
        let confirmation = swaplock
            .network_broadcast()
            .wait_for_broadcast_confirmation_timeout(transfer.pending, CALLBACK_TIMEOUT)
            .await?;

        println!(
            "confirmed #{}: amount={} {}, block={}, trx_num={}, id={}",
            transfer.index,
            transfer.amount,
            transfer.asset_id,
            confirmation.block_num(),
            confirmation.trx_num(),
            confirmation.id()
        );
    }

    println!("re-attaching dynamic_global_properties subscription handle on the same websocket");
    let mut subscription = swaplock
        .database()
        .dynamic_global_properties()
        .subscribe()
        .await?;
    println!(
        "subscription snapshot after transfers: head block {} {}",
        subscription.initial().head_block_number,
        subscription.initial().time
    );

    let update = subscription.next_update().await?;
    println!(
        "received dynamic_global_properties notice after transfer burst: head block {} {} | initial stress block {}",
        update.head_block_number, update.time, initial_head_block
    );

    Ok(())
}

struct PendingTransfer {
    index: usize,
    amount: i64,
    asset_id: String,
    pending: PendingBroadcastConfirmation,
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
