use std::env;
use std::error::Error;

use crate::rpc::GrapheneRpc;

pub const DEFAULT_RPC_URL: &str = "wss://node02.swaplock.chainpool.online:8090";
pub const GRAPHENE_RPC_URL_ENV: &str = "GRAPHENE_RPC_URL";

pub struct SwaplockSession {
    rpc: GrapheneRpc,
    database_api_id: u64,
    history_api_id: Option<u64>,
    network_broadcast_api_id: Option<u64>,
}

impl SwaplockSession {
    pub fn connect(url: &str) -> Result<Self, Box<dyn Error>> {
        let mut rpc = GrapheneRpc::connect(url)?;
        let database_api_id = rpc.database_api_id()?;

        Ok(Self {
            rpc,
            database_api_id,
            history_api_id: None,
            network_broadcast_api_id: None,
        })
    }

    pub fn connect_from_env_or_default() -> Result<Self, Box<dyn Error>> {
        let rpc_url =
            env::var(GRAPHENE_RPC_URL_ENV).unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
        Self::connect(&rpc_url)
    }

    pub fn rpc(&self) -> &GrapheneRpc {
        &self.rpc
    }

    pub fn rpc_mut(&mut self) -> &mut GrapheneRpc {
        &mut self.rpc
    }

    pub fn database_api_id(&self) -> u64 {
        self.database_api_id
    }

    pub fn history_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        if let Some(api_id) = self.history_api_id {
            return Ok(api_id);
        }

        let api_id = self.rpc.history_api_id()?;
        self.history_api_id = Some(api_id);
        Ok(api_id)
    }

    pub fn network_broadcast_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        if let Some(api_id) = self.network_broadcast_api_id {
            return Ok(api_id);
        }

        let api_id = self.rpc.network_broadcast_api_id()?;
        self.network_broadcast_api_id = Some(api_id);
        Ok(api_id)
    }
}
