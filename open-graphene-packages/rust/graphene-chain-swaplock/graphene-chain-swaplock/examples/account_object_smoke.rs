use std::env;
use std::error::Error;

use graphene_chain_swaplock::SwaplockSession;

fn main() -> Result<(), Box<dyn Error>> {
    let account_name =
        env::var("GRAPHENE_ACCOUNT_NAME").unwrap_or_else(|_| "committee-account".to_string());

    let mut session = SwaplockSession::connect_from_env_or_default()?;
    let database_api_id = session.database_api_id();
    let account_id = session.lookup_account_id(&account_name)?;
    let account = session
        .account_object(&account_id)?
        .ok_or_else(|| format!("account object {account_id} was not found"))?;

    println!("Connected: database_api_id={database_api_id}");
    println!("Account: {account_name} -> {account_id}");
    println!(
        "Account object: id={} name={} active_keys={} owner_keys={}",
        account.id.0,
        account.name,
        account.active.key_auths.len(),
        account.owner.key_auths.len()
    );

    Ok(())
}
