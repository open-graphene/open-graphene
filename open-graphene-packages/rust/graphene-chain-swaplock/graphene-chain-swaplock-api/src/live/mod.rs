mod chain_store;
mod database;
mod history;
mod market;
mod network_broadcast;

use std::time::{Duration, Instant};

use open_graphene_transport::{ApiIds, CallbackId, GrapheneSession, LiveTransport};

use crate::SwaplockApiError;

pub use chain_store::ChainStore;
pub use database::{
    SwaplockLiveAccountBalancesByIdRequest, SwaplockLiveAccountBalancesSubscription,
    SwaplockLiveAccountByIdRequest, SwaplockLiveAccountOrdersByIdRequest,
    SwaplockLiveAccountOrdersSubscription, SwaplockLiveAccountSubscription,
    SwaplockLiveAssetByIdRequest, SwaplockLiveAssetSubscription, SwaplockLiveDatabaseApi,
    SwaplockLiveDynamicGlobalPropertiesSubscription,
};
pub use history::{
    SwaplockLiveAccountHistoryByIdRequest, SwaplockLiveAccountHistoryRequest,
    SwaplockLiveAccountHistorySubscription, SwaplockLiveHistoryApi,
};
pub use market::SwaplockLiveMarketSubscription;
pub use network_broadcast::{
    SwaplockLiveNetworkBroadcastApi, SwaplockLivePendingBroadcastConfirmation,
};

pub struct SwaplockLiveApi {
    live: LiveTransport,
    api_ids: ApiIds,
    chain_id: String,
}

impl SwaplockLiveApi {
    pub(crate) fn from_session(session: GrapheneSession) -> Result<Self, SwaplockApiError> {
        let api_ids = session.api_ids().clone();
        let chain_id = session.chain_id().to_string();
        let live = session.into_live_transport()?;
        Ok(Self {
            live,
            api_ids,
            chain_id,
        })
    }

    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    pub fn api_ids(&self) -> &ApiIds {
        &self.api_ids
    }

    pub fn database(&self) -> SwaplockLiveDatabaseApi {
        SwaplockLiveDatabaseApi {
            live: self.live.handle(),
            database_api_id: self.api_ids.database,
        }
    }

    pub fn history(&self) -> Result<SwaplockLiveHistoryApi, SwaplockApiError> {
        let history_api_id = history_api_id(&self.api_ids)?;
        Ok(SwaplockLiveHistoryApi {
            live: self.live.handle(),
            database_api_id: self.api_ids.database,
            history_api_id,
        })
    }

    pub fn network_broadcast(&self) -> Result<SwaplockLiveNetworkBroadcastApi, SwaplockApiError> {
        let network_broadcast_api_id = network_broadcast_api_id(&self.api_ids)?;
        Ok(SwaplockLiveNetworkBroadcastApi {
            live: self.live.handle(),
            network_broadcast_api_id,
        })
    }
}

// Graphene's database API exposes one subscription callback per WebSocket/session.
// Typed database and history live subscriptions share this callback id, then
// multicast the raw notice stream in `LiveTransport` and filter/reconcile locally.
pub(super) const LIVE_DATABASE_CALLBACK_ID: u64 = 1;

fn remaining_or_callback_timeout(
    deadline: Instant,
    callback_id: CallbackId,
    timeout: Duration,
) -> Result<Duration, SwaplockApiError> {
    deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| {
            open_graphene_transport::TransportError::CallbackTimeout {
                callback_id,
                timeout,
            }
            .into()
        })
}

fn history_api_id(api_ids: &ApiIds) -> Result<u64, SwaplockApiError> {
    api_ids
        .history
        .ok_or(open_graphene_transport::TransportError::MissingApi { name: "history" }.into())
}

fn network_broadcast_api_id(api_ids: &ApiIds) -> Result<u64, SwaplockApiError> {
    api_ids.network_broadcast.ok_or(
        open_graphene_transport::TransportError::MissingApi {
            name: "network_broadcast",
        }
        .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_api_id_reports_missing_api() {
        let api_ids = ApiIds {
            database: 0,
            history: None,
            network_broadcast: Some(2),
            crypto: None,
            orders: None,
        };

        let error = history_api_id(&api_ids).unwrap_err();

        assert!(matches!(error, SwaplockApiError::Transport(_)));
        assert_eq!(
            error.to_string(),
            "Graphene API `history` is unavailable on this node"
        );
    }

    #[test]
    fn history_api_id_returns_available_api() {
        let api_ids = ApiIds {
            database: 0,
            history: Some(1),
            network_broadcast: Some(2),
            crypto: None,
            orders: None,
        };

        assert_eq!(history_api_id(&api_ids).unwrap(), 1);
    }

    #[test]
    fn network_broadcast_api_id_reports_missing_api() {
        let api_ids = ApiIds {
            database: 0,
            history: None,
            network_broadcast: None,
            crypto: None,
            orders: None,
        };

        let error = network_broadcast_api_id(&api_ids).unwrap_err();
        assert!(
            matches!(error, SwaplockApiError::Transport(open_graphene_transport::TransportError::MissingApi { name }) if name == "network_broadcast")
        );
    }

    #[test]
    fn network_broadcast_api_id_returns_available_api() {
        let api_ids = ApiIds {
            database: 0,
            history: None,
            network_broadcast: Some(4),
            crypto: None,
            orders: None,
        };

        assert_eq!(network_broadcast_api_id(&api_ids).unwrap(), 4);
    }
}
