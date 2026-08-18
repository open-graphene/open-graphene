//! Pedersen commitments are the building block of confidential transfers: they
//! hide an amount while still letting the network check the books balance.
//!
//! A commitment is `commit = blind·G + value·H`. Its key property is *homomorphism*:
//! adding two commitments equals committing to the sum of their values (with the
//! sum of their blinds). This example proves that property end-to-end:
//!
//!   commit(b1, 100) + commit(b2, 150)  ==  commit(b1+b2, 250)
//!
//! If that holds, a validator can confirm "inputs == outputs" without ever
//! seeing the amounts — which is exactly how a confidential transfer stays valid.

use graphene::{BlindingFactor, Graphene};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("1e9aea9e936607ca2cce053765c39c995c591ef23d85332ec63d6f06a0704f86")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    // Two 32-byte secret blinding factors (one per "input").
    let blind_a = BlindingFactor::from_hex(
        "1111111111111111111111111111111111111111111111111111111111111111",
    )?;
    let blind_b = BlindingFactor::from_hex(
        "2222222222222222222222222222222222222222222222222222222222222222",
    )?;
    let value_a = 100u64;
    let value_b = 150u64;

    // Commit to each value separately.
    let commit_a = swaplock
        .crypto()
        .blind(blind_a.clone(), value_a)
        .get()
        .await?;
    let commit_b = swaplock
        .crypto()
        .blind(blind_b.clone(), value_b)
        .get()
        .await?;
    println!("commit(b1, {value_a}) = {}", commit_a.to_hex());
    println!("commit(b2, {value_b}) = {}", commit_b.to_hex());

    // Commit to the total using the summed blinding factor.
    let blind_sum = swaplock
        .crypto()
        .blind_sum(vec![blind_a, blind_b], 2)
        .get()
        .await?;
    let commit_sum = swaplock
        .crypto()
        .blind(blind_sum, value_a + value_b)
        .get()
        .await?;
    println!(
        "commit(b1+b2, {}) = {}",
        value_a + value_b,
        commit_sum.to_hex()
    );

    // The node confirms: commit_a + commit_b - commit_sum == 0.
    let balances = swaplock
        .crypto()
        .verify_sum(vec![commit_a, commit_b], vec![commit_sum], 0)
        .get()
        .await?;

    println!("commit_a + commit_b == commit_sum ? {balances}");
    assert!(balances, "Pedersen commitments must be homomorphic");

    Ok(())
}
