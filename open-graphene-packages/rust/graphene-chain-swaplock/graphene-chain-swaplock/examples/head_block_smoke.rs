use std::error::Error;

use graphene_chain_swaplock::{database_api::head_block, SwaplockSession};

fn main() -> Result<(), Box<dyn Error>> {
    let mut session = SwaplockSession::connect_from_env_or_default()?;
    let database_api_id = session.database_api_id();
    let head = head_block(session.rpc_mut(), database_api_id)?;

    println!("Connected: database_api_id={database_api_id}");
    println!("Head block: {} {} {}", head.number, head.id, head.time);

    Ok(())
}
