//! Ergonomic builders for governance roles: register or update a committee member or witness, file
//! a worker proposal, or post a custom-payload operation.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). These are
//! privileged or stake-heavy ops; on a test chain they price and validate at `prepare()` even where
//! a full broadcast needs the right account or a large fee.

use graphene_chain_swaplock_bindings::generated::ids::{
    AccountId, AssetId, CommitteeMemberId, WitnessId,
};
use graphene_chain_swaplock_bindings::generated::operations::{
    CommitteeMemberCreateOperation, CommitteeMemberUpdateOperation, CustomOperation,
    WitnessCreateOperation, WitnessUpdateOperation, WorkerCreateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::{Operation, WorkerInitializer};
use graphene_chain_swaplock_bindings::generated::types::{
    Asset, BurnWorkerInitializer, RefundWorkerInitializer, VestingBalanceWorkerInitializer,
};
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Builder for `committee_member_create`: register `account` as a committee member.
pub struct CommitteeMemberCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    url: String,
}

impl<'session> CommitteeMemberCreateRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, account: impl Into<String>) -> Self {
        Self {
            session,
            account: account.into(),
            url: String::new(),
        }
    }

    /// A page describing the candidate.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::committee_member_create(CommitteeMemberCreateOperation {
            fee: core_fee(),
            committee_member_account: AccountId(self.account),
            url: self.url,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `committee_member_update`: change a committee member's page.
///
/// Required: the `committee_member` id and its `account`. Set `.url(..)` to change the page.
pub struct CommitteeMemberUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    committee_member: String,
    account: String,
    new_url: Option<String>,
}

impl<'session> CommitteeMemberUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        committee_member: impl Into<String>,
        account: impl Into<String>,
    ) -> Self {
        Self {
            session,
            committee_member: committee_member.into(),
            account: account.into(),
            new_url: None,
        }
    }

    /// New page describing the member.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.new_url = Some(url.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::committee_member_update(CommitteeMemberUpdateOperation {
            fee: core_fee(),
            committee_member: CommitteeMemberId(self.committee_member),
            committee_member_account: AccountId(self.account),
            new_url: self.new_url,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `witness_create`: register `account` as a witness.
///
/// Required: the `account` and its `block_signing_key` (a public key string). `.url(..)` adds a page.
pub struct WitnessCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    block_signing_key: String,
    url: String,
}

impl<'session> WitnessCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        block_signing_key: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            block_signing_key: block_signing_key.into(),
            url: String::new(),
        }
    }

    /// A page describing the witness.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::witness_create(WitnessCreateOperation {
            fee: core_fee(),
            witness_account: AccountId(self.account),
            url: self.url,
            block_signing_key: self.block_signing_key,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `witness_update`: change a witness's page or signing key.
///
/// Required: the `witness` id and its `account`. Set `.url(..)` and/or `.signing_key(..)`.
pub struct WitnessUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    witness: String,
    account: String,
    new_url: Option<String>,
    new_signing_key: Option<String>,
}

impl<'session> WitnessUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        witness: impl Into<String>,
        account: impl Into<String>,
    ) -> Self {
        Self {
            session,
            witness: witness.into(),
            account: account.into(),
            new_url: None,
            new_signing_key: None,
        }
    }

    /// New page describing the witness.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.new_url = Some(url.into());
        self
    }

    /// New block-signing public key.
    pub fn signing_key(mut self, key: impl Into<String>) -> Self {
        self.new_signing_key = Some(key.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::witness_update(WitnessUpdateOperation {
            fee: core_fee(),
            witness: WitnessId(self.witness),
            witness_account: AccountId(self.account),
            new_url: self.new_url,
            new_signing_key: self.new_signing_key,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `worker_create`: file a worker proposal that pays `daily_pay` between two dates.
///
/// Required: the `owner`, the `name`, the `.daily_pay(..)` and the work window
/// `.work_period(begin, end)`. The payout policy defaults to refund (unspent pay burns); choose
/// `.vesting(days)` or `.burn()` instead. Dates are `YYYY-MM-DDThh:mm:ss`.
pub struct WorkerCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    owner: String,
    name: String,
    daily_pay: i64,
    work_begin_date: String,
    work_end_date: String,
    url: String,
    initializer: WorkerInitializer,
}

impl<'session> WorkerCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            session,
            owner: owner.into(),
            name: name.into(),
            daily_pay: 0,
            work_begin_date: String::new(),
            work_end_date: String::new(),
            url: String::new(),
            initializer: WorkerInitializer::RefundWorkerInitializer(Box::new(
                RefundWorkerInitializer {},
            )),
        }
    }

    /// Daily pay in raw units of the core asset.
    pub fn daily_pay(mut self, daily_pay: i64) -> Self {
        self.daily_pay = daily_pay;
        self
    }

    /// The window the worker is funded for. Dates are `YYYY-MM-DDThh:mm:ss`.
    pub fn work_period(mut self, begin: impl Into<String>, end: impl Into<String>) -> Self {
        self.work_begin_date = begin.into();
        self.work_end_date = end.into();
        self
    }

    /// A page describing the worker.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Pay out gradually through a vesting balance over `pay_vesting_period_days`.
    pub fn vesting(mut self, pay_vesting_period_days: u16) -> Self {
        self.initializer = WorkerInitializer::VestingBalanceWorkerInitializer(Box::new(
            VestingBalanceWorkerInitializer {
                pay_vesting_period_days,
            },
        ));
        self
    }

    /// Burn the pay (a deflationary worker) instead of refunding or vesting it.
    pub fn burn(mut self) -> Self {
        self.initializer =
            WorkerInitializer::BurnWorkerInitializer(Box::new(BurnWorkerInitializer {}));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::worker_create(WorkerCreateOperation {
            fee: core_fee(),
            owner: AccountId(self.owner),
            work_begin_date: self.work_begin_date,
            work_end_date: self.work_end_date,
            daily_pay: self.daily_pay,
            name: self.name,
            url: self.url,
            initializer: self.initializer,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `custom`: post an opaque payload carried by the chain (for plugins and apps).
///
/// Required: the `payer` and the payload `.data(..)`. Add `.require_auth(..)` for each account whose
/// authority must sign, and set `.id(..)` to tag the payload type.
pub struct CustomRequest<'session> {
    session: &'session mut GrapheneSession,
    payer: String,
    required_auths: Vec<String>,
    id: u16,
    data: Vec<u8>,
}

impl<'session> CustomRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, payer: impl Into<String>) -> Self {
        Self {
            session,
            payer: payer.into(),
            required_auths: vec![],
            id: 0,
            data: vec![],
        }
    }

    /// An account whose active authority must sign the operation.
    pub fn require_auth(mut self, account: impl Into<String>) -> Self {
        self.required_auths.push(account.into());
        self
    }

    /// A tag identifying the payload type.
    pub fn id(mut self, id: u16) -> Self {
        self.id = id;
        self
    }

    /// The raw payload bytes.
    pub fn data(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.data = data.into();
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::custom(CustomOperation {
            fee: core_fee(),
            payer: AccountId(self.payer),
            required_auths: self.required_auths.into_iter().map(AccountId).collect(),
            id: self.id,
            data: self.data,
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
