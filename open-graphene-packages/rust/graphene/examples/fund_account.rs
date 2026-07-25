//! One-off dev helper: send core asset (1.3.0) from the registrar to an
//! account, mirroring the vault router's welcome transfer.
//!
//! Usage (env via infisical, prefix GRAPHENE_SWAPLOCK_VAULT_):
//!   cargo run --example fund_account -- <to-name-or-id> <amount-decimal>

use graphene::SwaplockApi;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let to = args.next().ok_or("usage: fund_account <to> <amount>")?;
    let amount = args.next().ok_or("usage: fund_account <to> <amount>")?;

    let rpc_servers = std::env::var("GRAPHENE_SWAPLOCK_VAULT_RPC_SERVERS")?;
    let registrar = std::env::var("GRAPHENE_SWAPLOCK_VAULT_REGISTRAR_ACCOUNT")?;
    let wif = std::env::var("GRAPHENE_SWAPLOCK_VAULT_REGISTRAR_WIF")?;

    let mut api = SwaplockApi::connect(rpc_servers.split(','), None).await?;

    let prepared = api
        .operations()
        .transfer()
        .from(&registrar)
        .to(&to)
        .amount_decimal(&amount, "1.3.0")
        .max_fee_raw(1_000_000)
        .prepare()
        .await?;
    let signed = api
        .operations()
        .sign_transfer_with_wif(prepared, &wif)
        .await?;
    let confirmation = api
        .network_broadcast()
        .broadcast_signed_transfer(signed)
        .await?;

    println!("sent {amount} core to {to}: {confirmation:?}");
    Ok(())
}
