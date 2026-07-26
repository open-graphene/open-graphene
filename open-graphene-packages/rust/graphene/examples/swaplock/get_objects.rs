use graphene::Graphene;

// Pull a mixed bag of chain objects by id in one call: the dynamic global properties (2.1.0), the
// core asset (1.3.0), and a made-up id to show that unknown ids come back as None. No keys needed,
// this is a plain read.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let ids = ["2.1.0", "1.3.0", "1.3.999999"];
    let objects = swaplock.database().objects(ids).get().await?;

    for (id, object) in ids.iter().zip(&objects) {
        match object {
            Some(object) => println!("{id}: {object:?}"),
            None => println!("{id}: not found"),
        }
    }
    Ok(())
}
