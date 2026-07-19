use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("f990ce83af5cf2d55c180ca4bd4b34161ccff2b2f8cc7d4987eea9555153930e")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let properties = swaplock.database().global_properties().get().await?;

    println!("id: {}", properties.id.0);
    println!("active witnesses: {}", properties.active_witnesses.len());
    println!(
        "active committee members: {}",
        properties.active_committee_members.len()
    );
    println!(
        "next available vote id: {}",
        properties.next_available_vote_id
    );
    println!(
        "pending parameters: {}",
        if properties.pending_parameters.is_some() {
            "yes"
        } else {
            "no"
        }
    );

    Ok(())
}
