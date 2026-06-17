//! Ergonomic builders for the issuer-side asset operations: issue, reserve and update.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). All are issuer
//! moves: `issue` mints new units of a user asset to an account, `reserve` burns units back out of
//! existence (only the holder can reserve their own), `update` changes the asset's options.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_swaplock_bindings::generated::operations::{
    AssetIssueOperation, AssetReserveOperation, AssetUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{Asset, AssetUpdateOperationExt};
use open_graphene_transport::GrapheneSession;

use crate::{DatabaseApi, SwaplockApiError};

use super::transaction::{PreparedTransaction, TransactionBuilder};

/// Builder for `asset_issue`: the asset's issuer mints `amount` and sends it to an account.
pub struct AssetIssueRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    asset_to_issue: Option<(i64, String)>,
    issue_to_account: Option<String>,
}

impl<'session> AssetIssueRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, issuer: impl Into<String>) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            asset_to_issue: None,
            issue_to_account: None,
        }
    }

    /// How much to mint: raw `amount` of asset id `asset_id` (must be an asset you issue).
    pub fn issue(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.asset_to_issue = Some((amount, asset_id.into()));
        self
    }

    /// Who receives the freshly minted units.
    pub fn to(mut self, account_id: impl Into<String>) -> Self {
        self.issue_to_account = Some(account_id.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset_id) = self
            .asset_to_issue
            .ok_or(SwaplockApiError::MissingTransferField { field: "issue" })?;
        let issue_to_account = self
            .issue_to_account
            .ok_or(SwaplockApiError::MissingTransferField { field: "to" })?;

        let operation = Operation::asset_issue(AssetIssueOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            issuer: AccountId(self.issuer),
            asset_to_issue: Asset::new(amount, AssetId(asset_id)),
            issue_to_account: AccountId(issue_to_account),
            memo: None,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_reserve`: burn `amount` you hold back out of the supply.
pub struct AssetReserveRequest<'session> {
    session: &'session mut GrapheneSession,
    payer: String,
    amount_to_reserve: Option<(i64, String)>,
}

impl<'session> AssetReserveRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, payer: impl Into<String>) -> Self {
        Self {
            session,
            payer: payer.into(),
            amount_to_reserve: None,
        }
    }

    /// How much to burn: raw `amount` of asset id `asset_id` from the payer's balance.
    pub fn amount(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.amount_to_reserve = Some((amount, asset_id.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset_id) = self
            .amount_to_reserve
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;

        let operation = Operation::asset_reserve(AssetReserveOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            payer: AccountId(self.payer),
            amount_to_reserve: Asset::new(amount, AssetId(asset_id)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_update`: the issuer changes a user asset's options in place.
///
/// The chain op replaces the whole options block, so this fetches the asset's current options
/// and applies only the fields you set, carrying the rest over. Use `.new_issuer(..)` to hand the
/// asset to another account. Bitasset-specific settings are out of scope here.
pub struct AssetUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    asset: String,
    new_issuer: Option<String>,
    description: Option<String>,
    max_supply: Option<i64>,
    market_fee_percent: Option<u16>,
    max_market_fee: Option<i64>,
    flags: Option<u16>,
}

impl<'session> AssetUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        issuer: impl Into<String>,
        asset: impl Into<String>,
    ) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            asset: asset.into(),
            new_issuer: None,
            description: None,
            max_supply: None,
            market_fee_percent: None,
            max_market_fee: None,
            flags: None,
        }
    }

    /// Hand the asset over to another account (its new issuer).
    pub fn new_issuer(mut self, account_id: impl Into<String>) -> Self {
        self.new_issuer = Some(account_id.into());
        self
    }

    /// Free-text description carried in the asset's options.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Cap on the total supply, in raw units.
    pub fn max_supply(mut self, max_supply: i64) -> Self {
        self.max_supply = Some(max_supply);
        self
    }

    /// Market fee taken on trades, in hundredths of a percent (e.g. `100` is 1%).
    pub fn market_fee_percent(mut self, market_fee_percent: u16) -> Self {
        self.market_fee_percent = Some(market_fee_percent);
        self
    }

    /// Cap on the market fee, in raw units.
    pub fn max_market_fee(mut self, max_market_fee: i64) -> Self {
        self.max_market_fee = Some(max_market_fee);
        self
    }

    /// The asset permission/behaviour flag bits.
    pub fn flags(mut self, flags: u16) -> Self {
        self.flags = Some(flags);
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let mut options = DatabaseApi {
            session: &mut *self.session,
        }
        .get_asset_by_id(&self.asset)
        .await?
        .options;

        if let Some(description) = self.description {
            options.description = description;
        }
        if let Some(max_supply) = self.max_supply {
            options.max_supply = max_supply;
        }
        if let Some(market_fee_percent) = self.market_fee_percent {
            options.market_fee_percent = market_fee_percent;
        }
        if let Some(max_market_fee) = self.max_market_fee {
            options.max_market_fee = max_market_fee;
        }
        if let Some(flags) = self.flags {
            options.flags = flags;
        }

        let operation = Operation::asset_update(AssetUpdateOperation {
            fee: Asset::new(0, AssetId("1.3.0".to_string())),
            issuer: AccountId(self.issuer),
            asset_to_update: AssetId(self.asset),
            new_issuer: self.new_issuer.map(AccountId),
            new_options: options,
            extensions: AssetUpdateOperationExt {},
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
