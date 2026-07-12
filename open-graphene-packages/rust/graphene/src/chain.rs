use std::marker::PhantomData;

use graphene_chain_swaplock_api::{
    ConnectionStrategy, ServerLatency, SwaplockApi, SwaplockLiveApi,
};

use crate::client::{GrapheneClientConfig, validate_config};
use crate::error::{GrapheneConfigError, GrapheneConnectError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swaplock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitShares;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Acta;

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

impl ChainClientBuilder<Swaplock> {
    pub async fn connect(self) -> Result<SwaplockApi, GrapheneConnectError> {
        validate_config(&self.config)?;
        let servers = self.config.servers;
        let expected_chain_id = self.config.chain_id;
        let strategy = self.config.strategy;
        Ok(
            SwaplockApi::connect_with_strategy(servers, expected_chain_id.as_deref(), strategy)
                .await?,
        )
    }

    pub async fn connect_live(self) -> Result<SwaplockLiveApi, GrapheneConnectError> {
        Ok(self.connect().await?.into_live()?)
    }

    /// Measure connect latency for every configured server, sorted fastest-first.
    ///
    /// Borrows the builder so you can inspect the report and then `connect()` separately.
    /// Unreachable servers are omitted from the report.
    pub async fn probe_latencies(&self) -> Result<Vec<ServerLatency>, GrapheneConnectError> {
        validate_config(&self.config)?;
        Ok(SwaplockApi::probe_latencies(
            self.config.servers.clone(),
            self.config.chain_id.as_deref(),
        )
        .await?)
    }
}

impl ChainClientBuilder<BitShares> {
    /// PLACEHOLDER: validates the config and returns a stub that holds it —
    /// no connection is made and no RPC surface exists yet. Swaplock is the
    /// only chain with a working client; see [`BitSharesClient`].
    pub async fn connect(self) -> Result<BitSharesClient, GrapheneConnectError> {
        validate_config(&self.config)?;
        Ok(BitSharesClient {
            config: self.config,
        })
    }
}

impl ChainClientBuilder<Acta> {
    /// PLACEHOLDER: validates the config and returns a stub that holds it —
    /// no connection is made and no RPC surface exists yet. Swaplock is the
    /// only chain with a working client; see [`ActaClient`].
    pub async fn connect(self) -> Result<ActaClient, GrapheneConnectError> {
        validate_config(&self.config)?;
        Ok(ActaClient {
            config: self.config,
        })
    }
}

/// PLACEHOLDER client: holds the validated config and nothing else. It does
/// not connect to any node and exposes no chain API. Kept so the multi-chain
/// builder surface is exercised until real BitShares support lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitSharesClient {
    config: GrapheneClientConfig,
}

impl BitSharesClient {
    pub fn config(&self) -> &GrapheneClientConfig {
        &self.config
    }
}

/// PLACEHOLDER client: holds the validated config and nothing else. It does
/// not connect to any node and exposes no chain API. Kept so the multi-chain
/// builder surface is exercised until real Acta support lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActaClient {
    config: GrapheneClientConfig,
}

impl ActaClient {
    pub fn config(&self) -> &GrapheneClientConfig {
        &self.config
    }
}
