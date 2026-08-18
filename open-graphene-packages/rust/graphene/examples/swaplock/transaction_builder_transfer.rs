use graphene::{AccountId, Asset, AssetId, Graphene, Operation, PrivateKey, TransferOperation};

// Send a transfer through the generic TransactionBuilder (not the transfer-specific helper):
// build a typed operation, let the node price it, sign, broadcast. The same path works for any
// operation the builder supports.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let wif = std::env::var("SWAPLOCK_ACTIVE_WIF")?;
    let from_id = std::env::var("SWAPLOCK_ACCOUNT_ID").unwrap_or_else(|_| "1.2.100".to_string());
    let public_key = PrivateKey::from_wif(&wif)?
        .to_public_key()
        .to_prefixed_string("BTS");

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("3b4f346481a66146d732eb22a43b5956134e5bea6124b379033589146036cbbf")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    // fee is left at 0; prepare() asks the node and fills it in.
    let transfer = Operation::transfer(TransferOperation {
        fee: Asset::new(0, AssetId("1.3.0".to_string())),
        from: AccountId(from_id),
        to: AccountId("1.2.0".to_string()),
        amount: Asset::new(1, AssetId("1.3.0".to_string())),
        memo: None,
        extensions: vec![],
    });

    let signed = swaplock
        .operations()
        .transaction()
        .add_operation(transfer)
        .prepare()
        .await?
        .sign_with_wif(&wif, &public_key)?;

    swaplock
        .network_broadcast()
        .broadcast_transaction(signed.transaction_json()?)
        .await?;
    println!("node accepted a transfer built via the generic TransactionBuilder");
    Ok(())
}
