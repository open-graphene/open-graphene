use std::error::Error;
use std::fmt;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrapheneConfigError {
    MissingServers,
}

impl fmt::Display for GrapheneConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingServers => {
                formatter.write_str("at least one Graphene RPC server is required")
            }
        }
    }
}

impl Error for GrapheneConfigError {}

#[derive(Debug)]
pub enum GrapheneConnectError {
    Config(GrapheneConfigError),
    Actanet(graphene_chain_actanet_api::ChainApiError),
    BitShares(graphene_chain_bitshares_api::ChainApiError),
    RSquared(graphene_chain_rsquared_api::ChainApiError),
}

impl fmt::Display for GrapheneConnectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Actanet(error) => error.fmt(formatter),
            Self::BitShares(error) => error.fmt(formatter),
            Self::RSquared(error) => error.fmt(formatter),
        }
    }
}

impl Error for GrapheneConnectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Actanet(error) => Some(error),
            Self::BitShares(error) => Some(error),
            Self::RSquared(error) => Some(error),
        }
    }
}

impl From<GrapheneConfigError> for GrapheneConnectError {
    fn from(error: GrapheneConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<graphene_chain_actanet_api::ChainApiError> for GrapheneConnectError {
    fn from(error: graphene_chain_actanet_api::ChainApiError) -> Self {
        Self::Actanet(error)
    }
}

impl From<graphene_chain_bitshares_api::ChainApiError> for GrapheneConnectError {
    fn from(error: graphene_chain_bitshares_api::ChainApiError) -> Self {
        Self::BitShares(error)
    }
}

impl From<graphene_chain_rsquared_api::ChainApiError> for GrapheneConnectError {
    fn from(error: graphene_chain_rsquared_api::ChainApiError) -> Self {
        Self::RSquared(error)
    }
}
