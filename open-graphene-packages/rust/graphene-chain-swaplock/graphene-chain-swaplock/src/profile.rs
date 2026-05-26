pub struct SwaplockProfile;

impl open_graphene_sdk_live::GrapheneChainProfile for SwaplockProfile {
    const CORE_ASSET_ID: &'static str = "1.3.0";
    const PUBLIC_KEY_PREFIX: &'static str = "BTS";

    fn expected_chain_id() -> Option<&'static str> {
        Some("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098")
    }
}
