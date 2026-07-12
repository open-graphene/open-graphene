//! Ergonomic builders for vesting balances: lock funds under a vesting policy, then withdraw the
//! vested portion.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). The policy
//! defaults to instant vesting (immediately withdrawable); use `.linear(..)` or `.cdd(..)` for a
//! real schedule.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, VestingBalanceId};
use graphene_chain_swaplock_bindings::generated::operations::{
    VestingBalanceCreateOperation, VestingBalanceWithdrawOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::{
    Operation, VestingPolicyInitializer,
};
use graphene_chain_swaplock_bindings::generated::types::{
    Asset, CddVestingPolicyInitializer, InstantVestingPolicyInitializer,
    LinearVestingPolicyInitializer,
};
use open_graphene_transport::GrapheneSession;

use crate::{DatabaseApi, SwaplockApiError};

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// How a vesting balance releases its funds.
enum Policy {
    Instant,
    Linear {
        begin_timestamp: Option<String>,
        cliff_seconds: u32,
        duration_seconds: u32,
    },
    Cdd {
        start_claim: Option<String>,
        vesting_seconds: u32,
    },
}

/// Builder for `vesting_balance_create`: lock `amount` for `owner` under a vesting policy.
///
/// Required: `creator`, `owner` and the `.amount(..)`. The policy defaults to instant (immediately
/// withdrawable); call `.linear(..)` or `.cdd(..)` for a schedule.
pub struct VestingBalanceCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    creator: String,
    owner: String,
    amount: Option<(i64, String)>,
    policy: Policy,
}

impl<'session> VestingBalanceCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        creator: impl Into<String>,
        owner: impl Into<String>,
    ) -> Self {
        Self {
            session,
            creator: creator.into(),
            owner: owner.into(),
            amount: None,
            policy: Policy::Instant,
        }
    }

    /// How much to lock, in raw units of `asset`.
    pub fn amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.amount = Some((amount, asset.into()));
        self
    }

    /// Linear vesting: nothing before `cliff_seconds`, then released evenly across
    /// `duration_seconds` from `begin_timestamp` (defaults to the head-block time).
    pub fn linear(mut self, cliff_seconds: u32, duration_seconds: u32) -> Self {
        self.policy = Policy::Linear {
            begin_timestamp: None,
            cliff_seconds,
            duration_seconds,
        };
        self
    }

    /// Coin-days-destroyed vesting over `vesting_seconds`, claimable from `start_claim`
    /// (defaults to the head-block time).
    pub fn cdd(mut self, vesting_seconds: u32) -> Self {
        self.policy = Policy::Cdd {
            start_claim: None,
            vesting_seconds,
        };
        self
    }

    /// Set the policy's start time explicitly (linear `begin_timestamp` or cdd `start_claim`).
    pub fn start_time(mut self, time: impl Into<String>) -> Self {
        let time = time.into();
        self.policy = match self.policy {
            Policy::Linear {
                cliff_seconds,
                duration_seconds,
                ..
            } => Policy::Linear {
                begin_timestamp: Some(time),
                cliff_seconds,
                duration_seconds,
            },
            Policy::Cdd {
                vesting_seconds, ..
            } => Policy::Cdd {
                start_claim: Some(time),
                vesting_seconds,
            },
            Policy::Instant => Policy::Instant,
        };
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let policy = build_policy(self.policy, self.session).await?;
        let operation = Operation::vesting_balance_create(VestingBalanceCreateOperation {
            fee: core_fee(),
            creator: AccountId(self.creator),
            owner: AccountId(self.owner),
            amount: Asset::new(amount, AssetId(asset)),
            policy,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Resolve a [`Policy`] into a binding initializer, filling in the head-block time for any unset
/// start time.
async fn build_policy(
    policy: Policy,
    session: &mut GrapheneSession,
) -> Result<VestingPolicyInitializer, SwaplockApiError> {
    Ok(match policy {
        Policy::Instant => VestingPolicyInitializer::InstantVestingPolicyInitializer(Box::new(
            InstantVestingPolicyInitializer {},
        )),
        Policy::Linear {
            begin_timestamp,
            cliff_seconds,
            duration_seconds,
        } => {
            let begin_timestamp = match begin_timestamp {
                Some(time) => time,
                None => head_block_time(session).await?,
            };
            VestingPolicyInitializer::LinearVestingPolicyInitializer(Box::new(
                LinearVestingPolicyInitializer {
                    begin_timestamp,
                    vesting_cliff_seconds: cliff_seconds,
                    vesting_duration_seconds: duration_seconds,
                },
            ))
        }
        Policy::Cdd {
            start_claim,
            vesting_seconds,
        } => {
            let start_claim = match start_claim {
                Some(time) => time,
                None => head_block_time(session).await?,
            };
            VestingPolicyInitializer::CddVestingPolicyInitializer(Box::new(
                CddVestingPolicyInitializer {
                    start_claim,
                    vesting_seconds,
                },
            ))
        }
    })
}

async fn head_block_time(session: &mut GrapheneSession) -> Result<String, SwaplockApiError> {
    Ok(DatabaseApi { session }
        .get_dynamic_global_properties()
        .await?
        .time)
}

/// Builder for `vesting_balance_withdraw`: claim vested funds back to the `owner`.
///
/// Required: the `vesting_balance` id, the `owner` and the `.amount(..)` (at most the vested portion).
pub struct VestingBalanceWithdrawRequest<'session> {
    session: &'session mut GrapheneSession,
    vesting_balance: String,
    owner: String,
    amount: Option<(i64, String)>,
}

impl<'session> VestingBalanceWithdrawRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        vesting_balance: impl Into<String>,
        owner: impl Into<String>,
    ) -> Self {
        Self {
            session,
            vesting_balance: vesting_balance.into(),
            owner: owner.into(),
            amount: None,
        }
    }

    /// How much to withdraw, in raw units of `asset`.
    pub fn amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.amount = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let operation = Operation::vesting_balance_withdraw(VestingBalanceWithdrawOperation {
            fee: core_fee(),
            vesting_balance: VestingBalanceId(self.vesting_balance),
            owner: AccountId(self.owner),
            amount: Asset::new(amount, AssetId(asset)),
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
