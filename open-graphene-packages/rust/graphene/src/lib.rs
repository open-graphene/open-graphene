mod chain;
mod client;
mod error;

pub use chain::{Actanet, BitShares, ChainClientBuilder, RSquared};
pub use client::{GrapheneClient, GrapheneClientBuilder, GrapheneClientConfig};
pub use error::{GrapheneConfigError, GrapheneConnectError};

/// The Actanet API, which is also re-exported at the crate root.
pub mod actanet {
    pub use graphene_chain_actanet_api::*;
}

/// The BitShares API.
pub mod bitshares {
    pub use graphene_chain_bitshares_api::*;
}

/// The R-Squared API. R-Squared lacks what BitShares 7 added (blind transfers, credit offers,
/// liquidity pools, samet funds and more), so neither do its builders.
pub mod rsquared {
    pub use graphene_chain_rsquared_api::*;
}

// Actanet is the default chain: its whole API sits at the crate root.
pub use graphene_chain_actanet_api::*;
pub use open_graphene_fc::hash;
pub use open_graphene_fc::{
    AccountKeys, Address, BrainKey, PrivateKey, PublicKey, SUGGESTED_BRAIN_KEY_WORDS, Signature,
    account_role_key, decrypt_with_checksum, encrypt_with_checksum,
};

pub struct Graphene;

impl Graphene {
    pub fn builder() -> GrapheneClientBuilder {
        GrapheneClientBuilder::new()
    }

    pub fn actanet() -> ChainClientBuilder<Actanet> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }

    pub fn bitshares() -> ChainClientBuilder<BitShares> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }

    pub fn rsquared() -> ChainClientBuilder<RSquared> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_builder_selects_actanet_api() {
        let builder = Graphene::builder()
            .servers(["wss://node01.example.org:8090"])
            .chain_id("chain-id")
            .prefix("ACTA")
            .build()
            .expect("valid generic Graphene config")
            .actanet();

        assert_eq!(
            builder.config().servers(),
            &["wss://node01.example.org:8090".to_string()]
        );
        assert_eq!(builder.config().chain_id(), Some("chain-id"));
        assert_eq!(builder.config().prefix(), Some("ACTA"));
    }

    #[test]
    fn chain_first_builder_collects_servers() {
        let builder = Graphene::actanet()
            .server("wss://node01.example.org:8090")
            .server("wss://node02.example.org:8090")
            .chain_id("chain-id")
            .prefix("ACTA")
            .build()
            .expect("valid Actanet config");

        assert_eq!(builder.config().servers().len(), 2);
        assert_eq!(builder.config().chain_id(), Some("chain-id"));
        assert_eq!(builder.config().prefix(), Some("ACTA"));
    }

    #[test]
    fn builder_rejects_missing_servers() {
        let error = Graphene::builder()
            .chain_id("chain-id")
            .prefix("ACTA")
            .build()
            .expect_err("servers are required");

        assert_eq!(error, GrapheneConfigError::MissingServers);
    }
}
