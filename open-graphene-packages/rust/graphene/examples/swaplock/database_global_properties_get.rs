use graphene::Graphene;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("95acb1f01e4afd0dd8b61f99377911a2f0ac6e6420925e01630f1d3580c89758")
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
