use std::error::Error;
use std::fmt;

use graphene_chain_swaplock_api::SwaplockApiError;

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
    SwaplockApi(SwaplockApiError),
}

impl fmt::Display for GrapheneConnectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::SwaplockApi(error) => error.fmt(formatter),
        }
    }
}

impl Error for GrapheneConnectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::SwaplockApi(error) => Some(error),
        }
    }
}

impl From<GrapheneConfigError> for GrapheneConnectError {
    fn from(error: GrapheneConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<SwaplockApiError> for GrapheneConnectError {
    fn from(error: SwaplockApiError) -> Self {
        Self::SwaplockApi(error)
    }
}
