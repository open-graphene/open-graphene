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
}

impl fmt::Display for GrapheneConnectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
        }
    }
}

impl Error for GrapheneConnectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
        }
    }
}

impl From<GrapheneConfigError> for GrapheneConnectError {
    fn from(error: GrapheneConfigError) -> Self {
        Self::Config(error)
    }
}
