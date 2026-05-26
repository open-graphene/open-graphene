use std::marker::PhantomData;

use graphene_chain_swaplock_api::SwaplockApi;

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
        Ok(SwaplockApi::connect(self.config.servers).await?)
    }
}

impl ChainClientBuilder<BitShares> {
    pub async fn connect(self) -> Result<BitSharesClient, GrapheneConnectError> {
        validate_config(&self.config)?;
        Ok(BitSharesClient {
            config: self.config,
        })
    }
}

impl ChainClientBuilder<Acta> {
    pub async fn connect(self) -> Result<ActaClient, GrapheneConnectError> {
        validate_config(&self.config)?;
        Ok(ActaClient {
            config: self.config,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitSharesClient {
    config: GrapheneClientConfig,
}

impl BitSharesClient {
    pub fn config(&self) -> &GrapheneClientConfig {
        &self.config
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActaClient {
    config: GrapheneClientConfig,
}

impl ActaClient {
    pub fn config(&self) -> &GrapheneClientConfig {
        &self.config
    }
}
