use crate::chain::{Acta, BitShares, ChainClientBuilder, Swaplock};
use crate::error::GrapheneConfigError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrapheneClientConfig {
    pub(crate) servers: Vec<String>,
    pub(crate) chain_id: Option<String>,
    pub(crate) prefix: Option<String>,
}

impl GrapheneClientConfig {
    pub fn servers(&self) -> &[String] {
        &self.servers
    }

    pub fn chain_id(&self) -> Option<&str> {
        self.chain_id.as_deref()
    }

    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrapheneClientBuilder {
    servers: Vec<String>,
    chain_id: Option<String>,
    prefix: Option<String>,
}

impl GrapheneClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn server(mut self, server: impl Into<String>) -> Self {
        self.servers.push(server.into());
        self
    }

    pub fn servers<I, S>(mut self, servers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.servers.extend(servers.into_iter().map(Into::into));
        self
    }

    pub fn chain_id(mut self, chain_id: impl Into<String>) -> Self {
        self.chain_id = Some(chain_id.into());
        self
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn build(self) -> Result<GrapheneClient, GrapheneConfigError> {
        let config = GrapheneClientConfig {
            servers: self.servers,
            chain_id: self.chain_id,
            prefix: self.prefix,
        };
        validate_config(&config)?;
        Ok(GrapheneClient { config })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrapheneClient {
    config: GrapheneClientConfig,
}

impl GrapheneClient {
    pub fn config(&self) -> &GrapheneClientConfig {
        &self.config
    }

    pub fn swaplock(self) -> ChainClientBuilder<Swaplock> {
        ChainClientBuilder::from_config(self.config)
    }

    pub fn bitshares(self) -> ChainClientBuilder<BitShares> {
        ChainClientBuilder::from_config(self.config)
    }

    pub fn acta(self) -> ChainClientBuilder<Acta> {
        ChainClientBuilder::from_config(self.config)
    }
}

pub(crate) fn validate_config(config: &GrapheneClientConfig) -> Result<(), GrapheneConfigError> {
    if config.servers.is_empty() {
        return Err(GrapheneConfigError::MissingServers);
    }
    Ok(())
}
