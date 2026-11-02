use std::marker::PhantomData;

use graphene_chain_actanet_api::{ConnectionStrategy, ServerLatency};

use crate::client::{GrapheneClientConfig, validate_config};
use crate::error::{GrapheneConfigError, GrapheneConnectError};

/// Actanet: the default chain of this facade, whose API is also re-exported at the crate root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actanet;

/// BitShares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitShares;

/// R-Squared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RSquared;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainClientBuilder<C> {
    config: GrapheneClientConfig,
    chain: PhantomData<C>,
}

impl<C> ChainClientBuilder<C> {
    pub(crate) fn from_config(config: GrapheneClientConfig) -> Self {
        Self {
            config,
            chain: PhantomData,
        }
    }

    pub fn server(mut self, server: impl Into<String>) -> Self {
        self.config.servers.push(server.into());
        self
    }

    pub fn servers<I, S>(mut self, servers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config
            .servers
            .extend(servers.into_iter().map(Into::into));
        self
    }

    pub fn chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.config.chain_id = Some(chain_id.into());
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.prefix = Some(prefix.into());
        self
    }

    pub fn strategy(mut self, strategy: ConnectionStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }

    pub fn lowest_latency(self) -> Self {
        self.strategy(ConnectionStrategy::LowestLatency)
    }

    pub fn config(&self) -> &GrapheneClientConfig {
        &self.config
    }

    pub fn build(self) -> Result<Self, GrapheneConfigError> {
        validate_config(&self.config)?;
        Ok(self)
    }
}

/// `connect`, `connect_live` and `probe_latencies` for one chain, built from its API crate.
macro_rules! chain_connect {
    ($marker:ty, $api:ident) => {
        impl ChainClientBuilder<$marker> {
            pub async fn connect(self) -> Result<$api::ChainApi, GrapheneConnectError> {
                validate_config(&self.config)?;
                let servers = self.config.servers;
                let expected_chain_id = self.config.chain_id;
                let strategy = self.config.strategy;
                Ok(
                    $api::ChainApi::connect_with_strategy(
                        servers,
                        expected_chain_id.as_deref(),
                        strategy,
                    )
                    .await?,
                )
            }

            pub async fn connect_live(self) -> Result<$api::ChainLiveApi, GrapheneConnectError> {
                Ok(self.connect().await?.into_live()?)
            }

            /// Measure connect latency for every configured server, sorted fastest-first.
            ///
            /// Borrows the builder so you can inspect the report and then `connect()` separately.
            /// Unreachable servers are omitted from the report.
            pub async fn probe_latencies(&self) -> Result<Vec<ServerLatency>, GrapheneConnectError> {
                validate_config(&self.config)?;
                Ok($api::ChainApi::probe_latencies(
                    self.config.servers.clone(),
                    self.config.chain_id.as_deref(),
                )
                .await?)
            }
        }
    };
}

chain_connect!(Actanet, graphene_chain_actanet_api);
chain_connect!(BitShares, graphene_chain_bitshares_api);
chain_connect!(RSquared, graphene_chain_rsquared_api);
