use std::env;
use std::error::Error;

use graphene_chain_swaplock_bindings::generated::{
    AccountObject, AssetDynamicDataObject, AssetObject, OperationHistoryObject,
};
use open_graphene_sdk_core::HeadBlock;

use crate::{database_api, history_api, rpc::GrapheneRpc};

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

    pub fn head_block(&mut self) -> Result<HeadBlock, Box<dyn Error>> {
        database_api::head_block(&mut self.rpc, self.database_api_id)
    }

    pub fn lookup_account_id(&mut self, account_name: &str) -> Result<String, Box<dyn Error>> {
        database_api::lookup_account_id(&mut self.rpc, self.database_api_id, account_name)
    }

    pub fn lookup_account_id_optional(
        &mut self,
        account_name: &str,
    ) -> Result<Option<String>, Box<dyn Error>> {
        database_api::lookup_account_id_optional(&mut self.rpc, self.database_api_id, account_name)
    }

    pub fn lookup_asset_id(&mut self, symbol: &str) -> Result<String, Box<dyn Error>> {
        database_api::lookup_asset_id(&mut self.rpc, self.database_api_id, symbol)
    }

    pub fn lookup_asset_id_optional(
        &mut self,
        symbol: &str,
    ) -> Result<Option<String>, Box<dyn Error>> {
        database_api::lookup_asset_id_optional(&mut self.rpc, self.database_api_id, symbol)
    }

    pub fn account_balance(
        &mut self,
        account_id: &str,
        asset_id: &str,
    ) -> Result<i64, Box<dyn Error>> {
        database_api::account_balance(&mut self.rpc, self.database_api_id, account_id, asset_id)
    }

    pub fn account_object(
        &mut self,
        account_id: &str,
    ) -> Result<Option<AccountObject>, Box<dyn Error>> {
        database_api::account_object(&mut self.rpc, self.database_api_id, account_id)
    }

    pub fn asset_object(&mut self, asset_id: &str) -> Result<Option<AssetObject>, Box<dyn Error>> {
        database_api::asset_object(&mut self.rpc, self.database_api_id, asset_id)
    }

    pub fn asset_dynamic_data_object(
        &mut self,
        asset_dynamic_data_id: &str,
    ) -> Result<Option<AssetDynamicDataObject>, Box<dyn Error>> {
        database_api::asset_dynamic_data_object(
            &mut self.rpc,
            self.database_api_id,
            asset_dynamic_data_id,
        )
    }

    pub fn account_history_objects(
        &mut self,
        query: &history_api::AccountHistoryQuery<'_>,
    ) -> Result<Vec<OperationHistoryObject>, Box<dyn Error>> {
        let history_api_id = self.history_api_id()?;
        history_api::get_account_history_objects(&mut self.rpc, history_api_id, query)
    }
}
