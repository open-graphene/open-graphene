//! Ergonomic builders for withdraw permissions: an account authorises another to pull a capped
//! amount on a recurring schedule, the authorised account claims against it, and either side can
//! update or delete it.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder).

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, WithdrawPermissionId};
use graphene_chain_swaplock_bindings::generated::operations::{
    WithdrawPermissionClaimOperation, WithdrawPermissionCreateOperation,
    WithdrawPermissionDeleteOperation, WithdrawPermissionUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_core::expiration_from_head_time;
use open_graphene_transport::GrapheneSession;

use crate::{DatabaseApi, SwaplockApiError};

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

/// One day, the default withdrawal period when the caller does not set one.
const DEFAULT_PERIOD_SEC: u32 = 24 * 60 * 60;

/// Default number of periods a fresh permission runs for.
const DEFAULT_PERIODS_UNTIL_EXPIRATION: u32 = 12;

/// The chain requires the first period to start in the future, so the default sits a few minutes
/// past the head block.
const DEFAULT_START_AHEAD: Duration = Duration::from_secs(5 * 60);

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// The default first-period start: a few minutes past the head-block time.
async fn default_start_time(session: &mut GrapheneSession) -> Result<String, SwaplockApiError> {
    let properties = DatabaseApi { session }
        .get_dynamic_global_properties()
        .await?;
    Ok(expiration_from_head_time(
        &properties.time,
        DEFAULT_START_AHEAD,
    )?)
}

/// Builder for `withdraw_permission_create`: let `authorized_account` pull from `withdraw_from`.
///
/// Required: `withdraw_from`, the `authorized_account` and the per-period `.limit(..)`. The schedule
/// defaults to twelve daily periods starting a few minutes past the head block (the chain requires a
/// future start); override with the setters.
pub struct WithdrawPermissionCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    withdraw_from: String,
    authorized_account: String,
    limit: Option<(i64, String)>,
    period_sec: u32,
    periods_until_expiration: u32,
    period_start_time: Option<String>,
}

impl<'session> WithdrawPermissionCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        withdraw_from: impl Into<String>,
        authorized_account: impl Into<String>,
    ) -> Self {
        Self {
            session,
            withdraw_from: withdraw_from.into(),
            authorized_account: authorized_account.into(),
            limit: None,
            period_sec: DEFAULT_PERIOD_SEC,
            periods_until_expiration: DEFAULT_PERIODS_UNTIL_EXPIRATION,
            period_start_time: None,
        }
    }

    /// The most the authorised account may pull per period, in raw units of `asset`.
    pub fn limit(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.limit = Some((amount, asset.into()));
        self
    }

    /// Length of one withdrawal period in seconds (default one day).
    pub fn period_sec(mut self, period_sec: u32) -> Self {
        self.period_sec = period_sec;
        self
    }

    /// How many periods the permission lasts (default twelve).
    pub fn periods_until_expiration(mut self, periods: u32) -> Self {
        self.periods_until_expiration = periods;
        self
    }

    /// When the first period starts (defaults to the head-block time). Format: `YYYY-MM-DDThh:mm:ss`.
    pub fn period_start_time(mut self, time: impl Into<String>) -> Self {
        self.period_start_time = Some(time.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .limit
            .ok_or(SwaplockApiError::MissingTransferField { field: "limit" })?;
        let period_start_time = match self.period_start_time {
            Some(time) => time,
            None => default_start_time(self.session).await?,
        };
        let operation = Operation::withdraw_permission_create(WithdrawPermissionCreateOperation {
            fee: core_fee(),
            withdraw_from_account: AccountId(self.withdraw_from),
            authorized_account: AccountId(self.authorized_account),
            withdrawal_limit: Asset::new(amount, AssetId(asset)),
            withdrawal_period_sec: self.period_sec,
            periods_until_expiration: self.periods_until_expiration,
            period_start_time,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `withdraw_permission_update`: change a live permission in place.
///
/// Required: `withdraw_from`, the `authorized_account`, the `permission` id, the new per-period
/// `.limit(..)` and `.period_start_time(..)`. The schedule fields default as for create.
pub struct WithdrawPermissionUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    withdraw_from: String,
    authorized_account: String,
    permission: String,
    limit: Option<(i64, String)>,
    period_sec: u32,
    periods_until_expiration: u32,
    period_start_time: Option<String>,
}

impl<'session> WithdrawPermissionUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        withdraw_from: impl Into<String>,
        authorized_account: impl Into<String>,
        permission: impl Into<String>,
    ) -> Self {
        Self {
            session,
            withdraw_from: withdraw_from.into(),
            authorized_account: authorized_account.into(),
            permission: permission.into(),
            limit: None,
            period_sec: DEFAULT_PERIOD_SEC,
            periods_until_expiration: DEFAULT_PERIODS_UNTIL_EXPIRATION,
            period_start_time: None,
        }
    }

    /// New per-period cap, in raw units of `asset`.
    pub fn limit(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.limit = Some((amount, asset.into()));
        self
    }

    /// New withdrawal period length in seconds.
    pub fn period_sec(mut self, period_sec: u32) -> Self {
        self.period_sec = period_sec;
        self
    }

    /// New number of periods until the permission expires.
    pub fn periods_until_expiration(mut self, periods: u32) -> Self {
        self.periods_until_expiration = periods;
        self
    }

    /// New period start time (defaults to the head-block time).
    pub fn period_start_time(mut self, time: impl Into<String>) -> Self {
        self.period_start_time = Some(time.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .limit
            .ok_or(SwaplockApiError::MissingTransferField { field: "limit" })?;
        let period_start_time = match self.period_start_time {
            Some(time) => time,
            None => default_start_time(self.session).await?,
        };
        let operation = Operation::withdraw_permission_update(WithdrawPermissionUpdateOperation {
            fee: core_fee(),
            withdraw_from_account: AccountId(self.withdraw_from),
            authorized_account: AccountId(self.authorized_account),
            permission_to_update: WithdrawPermissionId(self.permission),
            withdrawal_limit: Asset::new(amount, AssetId(asset)),
            withdrawal_period_sec: self.period_sec,
            period_start_time,
            periods_until_expiration: self.periods_until_expiration,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `withdraw_permission_claim`: the authorised account pulls funds.
///
/// Required: the `permission` id, the `withdraw_from` and `withdraw_to` accounts and the
/// `.amount(..)` to pull (within the per-period limit).
pub struct WithdrawPermissionClaimRequest<'session> {
    session: &'session mut GrapheneSession,
    permission: String,
    withdraw_from: String,
    withdraw_to: String,
    amount: Option<(i64, String)>,
}

impl<'session> WithdrawPermissionClaimRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        permission: impl Into<String>,
        withdraw_from: impl Into<String>,
        withdraw_to: impl Into<String>,
    ) -> Self {
        Self {
            session,
            permission: permission.into(),
            withdraw_from: withdraw_from.into(),
            withdraw_to: withdraw_to.into(),
            amount: None,
        }
    }

    /// How much to pull, in raw units of `asset`.
    pub fn amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.amount = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let operation = Operation::withdraw_permission_claim(WithdrawPermissionClaimOperation {
            fee: core_fee(),
            withdraw_permission: WithdrawPermissionId(self.permission),
            withdraw_from_account: AccountId(self.withdraw_from),
            withdraw_to_account: AccountId(self.withdraw_to),
            amount_to_withdraw: Asset::new(amount, AssetId(asset)),
            memo: None,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `withdraw_permission_delete`: revoke a permission.
pub struct WithdrawPermissionDeleteRequest<'session> {
    session: &'session mut GrapheneSession,
    withdraw_from: String,
    authorized_account: String,
    permission: String,
}

impl<'session> WithdrawPermissionDeleteRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        withdraw_from: impl Into<String>,
        authorized_account: impl Into<String>,
        permission: impl Into<String>,
    ) -> Self {
        Self {
            session,
            withdraw_from: withdraw_from.into(),
            authorized_account: authorized_account.into(),
            permission: permission.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::withdraw_permission_delete(WithdrawPermissionDeleteOperation {
            fee: core_fee(),
            withdraw_from_account: AccountId(self.withdraw_from),
            authorized_account: AccountId(self.authorized_account),
            withdrawal_permission: WithdrawPermissionId(self.permission),
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
