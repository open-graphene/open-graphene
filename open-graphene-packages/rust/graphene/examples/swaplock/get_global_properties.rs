use graphene::Graphene;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut swaplock = Graphene::builder()
        .servers([
            "wss://node01.swaplock.chainpool.online:8090",
            "wss://node02.swaplock.chainpool.online:8090",
        ])
        .chain_id("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
        .prefix("BTS")
        .build()?
        .swaplock()
        .connect()
        .await?;

    let properties = swaplock.database().get_global_properties().await?;

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
