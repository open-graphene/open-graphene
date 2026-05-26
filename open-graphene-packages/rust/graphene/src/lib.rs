mod chain;
mod client;
mod error;

pub use chain::{Acta, ActaClient, BitShares, BitSharesClient, ChainClientBuilder, Swaplock};
pub use client::{GrapheneClient, GrapheneClientBuilder, GrapheneClientConfig};
pub use error::{GrapheneConfigError, GrapheneConnectError};
pub use graphene_chain_swaplock_api::{SWAPLOCK_CHAIN_ID, SwaplockApi, SwaplockApiError};

pub struct Graphene;

impl Graphene {
    pub fn builder() -> GrapheneClientBuilder {
        GrapheneClientBuilder::new()
    }

    pub fn swaplock() -> ChainClientBuilder<Swaplock> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }

    pub fn bitshares() -> ChainClientBuilder<BitShares> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }

    pub fn acta() -> ChainClientBuilder<Acta> {
        ChainClientBuilder::from_config(GrapheneClientConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_builder_selects_swaplock_api() {
        let builder = Graphene::builder()
            .servers(["wss://node01.swaplock.chainpool.online:8090"])
            .chain_id("chain-id")
            .prefix("BTS")
            .build()
            .expect("valid generic Graphene config")
            .swaplock();

        assert_eq!(
            builder.config().servers(),
            &["wss://node01.swaplock.chainpool.online:8090".to_string()]
        );
        assert_eq!(builder.config().chain_id(), Some("chain-id"));
        assert_eq!(builder.config().prefix(), Some("BTS"));
    }

    #[test]
    fn chain_first_builder_collects_servers() {
        let builder = Graphene::swaplock()
            .server("wss://node01.swaplock.chainpool.online:8090")
            .server("wss://node02.swaplock.chainpool.online:8090")
            .chain_id("chain-id")
            .prefix("BTS")
            .build()
            .expect("valid Swaplock config");

        assert_eq!(builder.config().servers().len(), 2);
        assert_eq!(builder.config().chain_id(), Some("chain-id"));
        assert_eq!(builder.config().prefix(), Some("BTS"));
    }

    #[test]
    fn builder_rejects_missing_servers() {
        let error = Graphene::builder()
            .chain_id("chain-id")
            .prefix("BTS")
            .build()
            .expect_err("servers are required");

        assert_eq!(error, GrapheneConfigError::MissingServers);
    }
}
