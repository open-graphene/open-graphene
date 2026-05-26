use std::error::Error;

use open_graphene_transport::{
    broadcast_transaction as transport_broadcast_transaction,
    get_account_history as transport_get_account_history, get_objects as transport_get_objects,
    get_required_fees as transport_get_required_fees,
    AccountHistoryQuery as TransportAccountHistoryQuery, GrapheneSession,
};
use serde_json::Value;

pub struct GrapheneRpc {
    session: GrapheneSession,
}

impl GrapheneRpc {
    pub fn connect(url: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            session: GrapheneSession::connect(url)?,
        })
    }

    pub fn database_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        Ok(self.session.api_ids().database)
    }

    pub fn network_broadcast_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        self.session
            .api_ids()
            .network_broadcast
            .ok_or_else(|| "network_broadcast API is unavailable".into())
    }

    pub fn history_api_id(&mut self) -> Result<u64, Box<dyn Error>> {
        self.session
            .api_ids()
            .history
            .ok_or_else(|| "history API is unavailable".into())
    }

    pub fn call_database(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        ensure_api_id("database", self.session.api_ids().database, api_id)?;
        Ok(self.session.database_call(method, params)?)
    }

    pub fn get_objects<I, S>(&mut self, api_id: u64, ids: I) -> Result<Value, Box<dyn Error>>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        ensure_api_id("database", self.session.api_ids().database, api_id)?;
        Ok(transport_get_objects(&mut self.session, ids)?)
    }

    pub fn get_required_fees(
        &mut self,
        api_id: u64,
        operations_json: Value,
        fee_asset_id: impl Into<String>,
    ) -> Result<Value, Box<dyn Error>> {
        ensure_api_id("database", self.session.api_ids().database, api_id)?;
        Ok(transport_get_required_fees(
            &mut self.session,
            operations_json,
            fee_asset_id,
        )?)
    }

    pub fn call_network_broadcast(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        let expected = self
            .session
            .api_ids()
            .network_broadcast
            .ok_or("network_broadcast API is unavailable")?;
        ensure_api_id("network_broadcast", expected, api_id)?;
        Ok(self.session.network_broadcast_call(method, params)?)
    }

    pub fn broadcast_transaction(
        &mut self,
        api_id: u64,
        transaction_json: Value,
    ) -> Result<(), Box<dyn Error>> {
        let expected = self
            .session
            .api_ids()
            .network_broadcast
            .ok_or("network_broadcast API is unavailable")?;
        ensure_api_id("network_broadcast", expected, api_id)?;
        Ok(transport_broadcast_transaction(
            &mut self.session,
            transaction_json,
        )?)
    }

    pub fn call_history(
        &mut self,
        api_id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        let expected = self
            .session
            .api_ids()
            .history
            .ok_or("history API is unavailable")?;
        ensure_api_id("history", expected, api_id)?;
        Ok(self.session.history_call(method, params)?)
    }

    pub fn get_account_history(
        &mut self,
        api_id: u64,
        query: &TransportAccountHistoryQuery,
    ) -> Result<Value, Box<dyn Error>> {
        let expected = self
            .session
            .api_ids()
            .history
            .ok_or("history API is unavailable")?;
        ensure_api_id("history", expected, api_id)?;
        Ok(transport_get_account_history(&mut self.session, query)?)
    }
}

fn ensure_api_id(name: &str, expected: u64, actual: u64) -> Result<(), Box<dyn Error>> {
    if expected != actual {
        return Err(format!("{name} API id mismatch: expected {expected}, got {actual}").into());
    }
    Ok(())
}
