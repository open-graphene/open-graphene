#[path = "common.rs"]
mod common;

use graphene::{Graphene, PrivateKey};

// Exercise the confidential (blind) transfer family.
//
// These ops only apply when the input commitments plus the fee balance the outputs, and actanet
// does not expose the crypto API needed to build real commitments here, so we node-verify each at
// prepare() (the node prices the operation) without broadcasting. The commitment and range-proof
// bytes below are placeholders: in real use you build them with the crypto API (see the
// crypto_pedersen_commitment example) and hand the finished bytes to these thin builders.
const CORE: &str = "1.3.0";
const VALUE: i64 = 100_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("ACTANET_ACTIVE_WIF")?;
    let account = std::env::var("ACTANET_ACCOUNT").unwrap_or_else(|_| "actanet".to_string());
    let owner_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string(graphene::PUBLIC_KEY_PREFIX);

    let mut actanet = Graphene::builder()
        .servers(common::servers())
        .prefix(graphene::PUBLIC_KEY_PREFIX)
        .build()?
        .actanet()
        .connect()
        .await?;

    let id = actanet
        .database()
        .account_by_name(&account)
        .get()
        .await?
        .id
        .0;

    // Placeholder confidential payload: a 32-byte blinding factor, a 33-byte commitment and a
    // range proof. Build these for real with the crypto API; here they only have to price.
    let blinding_factor = vec![0x11u8; 32];
    let commitment = {
        let mut bytes = vec![0x08u8];
        bytes.extend(std::iter::repeat_n(0x22u8, 32));
        bytes
    };
    let range_proof = vec![0x33u8; 200];

    // transfer_to_blind: a public amount in, one blind output back to our own key.
    actanet
        .operations()
        .transfer_to_blind(&id, VALUE, CORE)
        .blinding_factor(blinding_factor.clone())
        .output(commitment.clone(), range_proof.clone(), &owner_key)
        .prepare()
        .await?;

    // blind_transfer: spend one commitment, produce one.
    actanet
        .operations()
        .blind_transfer()
        .input(commitment.clone(), &owner_key)
        .output(commitment.clone(), range_proof.clone(), &owner_key)
        .prepare()
        .await?;

    // transfer_from_blind: a blind input out to a public amount.
    actanet
        .operations()
        .transfer_from_blind(&id, VALUE, CORE)
        .blinding_factor(blinding_factor)
        .input(commitment, &owner_key)
        .prepare()
        .await?;

    println!(
        "transfer_to_blind / blind_transfer / transfer_from_blind: node-priced (not broadcast)"
    );
    Ok(())
}
