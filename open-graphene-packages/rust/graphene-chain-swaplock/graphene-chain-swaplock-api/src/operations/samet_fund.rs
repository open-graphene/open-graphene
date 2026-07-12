//! Ergonomic builders for the SameT-fund family: a lender posts a fund, and a borrower takes a
//! flash loan that must be repaid inside the same transaction.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). `borrow` and
//! `repay` only validate as a pair in one transaction; the standalone builders here are for pricing
//! and for composing into a `TransactionBuilder` yourself.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, SametFundId};
use graphene_chain_swaplock_bindings::generated::operations::{
    SametFundBorrowOperation, SametFundCreateOperation, SametFundDeleteOperation,
    SametFundRepayOperation, SametFundUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Builder for `samet_fund_create`: post funds for flash-loan borrowing.
///
/// Required: `owner_account`, the `asset_type` to lend and the `balance` to commit. `.fee_rate(..)`
/// sets the flash-loan fee (defaults to none).
pub struct SametFundCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    owner_account: String,
    asset_type: String,
    balance: i64,
    fee_rate: u32,
}

impl<'session> SametFundCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        owner_account: impl Into<String>,
        asset_type: impl Into<String>,
        balance: i64,
    ) -> Self {
        Self {
            session,
            owner_account: owner_account.into(),
            asset_type: asset_type.into(),
            balance,
            fee_rate: 0,
        }
    }

    /// Flash-loan fee charged on a borrow, in millionths (1000000 = 100%).
    pub fn fee_rate(mut self, fee_rate: u32) -> Self {
        self.fee_rate = fee_rate;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::samet_fund_create(SametFundCreateOperation {
            fee: core_fee(),
            owner_account: AccountId(self.owner_account),
            asset_type: AssetId(self.asset_type),
            balance: self.balance,
            fee_rate: self.fee_rate,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `samet_fund_update`: change a live fund in place.
///
/// Required: `owner_account` and the `fund_id`. `.delta_amount(..)` tops up (positive) or withdraws
/// (negative) the committed balance; `.fee_rate(..)` changes the flash-loan fee. Both are partial.
pub struct SametFundUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    owner_account: String,
    fund_id: String,
    delta_amount: Option<(i64, String)>,
    new_fee_rate: Option<u32>,
}

impl<'session> SametFundUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        owner_account: impl Into<String>,
        fund_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            owner_account: owner_account.into(),
            fund_id: fund_id.into(),
            delta_amount: None,
            new_fee_rate: None,
        }
    }

    /// Top up (positive) or withdraw (negative) committed balance, in raw units of `asset`.
    pub fn delta_amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.delta_amount = Some((amount, asset.into()));
        self
    }

    /// New flash-loan fee, in millionths.
    pub fn fee_rate(mut self, fee_rate: u32) -> Self {
        self.new_fee_rate = Some(fee_rate);
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::samet_fund_update(SametFundUpdateOperation {
            fee: core_fee(),
            owner_account: AccountId(self.owner_account),
            fund_id: SametFundId(self.fund_id),
            delta_amount: self
                .delta_amount
                .map(|(amount, asset)| Asset::new(amount, AssetId(asset))),
            new_fee_rate: self.new_fee_rate,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `samet_fund_delete`: withdraw a fund and reclaim its balance.
pub struct SametFundDeleteRequest<'session> {
    session: &'session mut GrapheneSession,
    owner_account: String,
    fund_id: String,
}

impl<'session> SametFundDeleteRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        owner_account: impl Into<String>,
        fund_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            owner_account: owner_account.into(),
            fund_id: fund_id.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::samet_fund_delete(SametFundDeleteOperation {
            fee: core_fee(),
            owner_account: AccountId(self.owner_account),
            fund_id: SametFundId(self.fund_id),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `samet_fund_borrow`: take a flash loan from a fund.
///
/// Required: the `borrower`, the `fund_id` and the `.amount(..)` to borrow. The loan must be repaid
/// with a `samet_fund_repay` in the **same transaction**, so compose both into one
/// [`TransactionBuilder`](super::transaction::TransactionBuilder) rather than broadcasting alone.
pub struct SametFundBorrowRequest<'session> {
    session: &'session mut GrapheneSession,
    borrower: String,
    fund_id: String,
    borrow_amount: Option<(i64, String)>,
}

impl<'session> SametFundBorrowRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        borrower: impl Into<String>,
        fund_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            borrower: borrower.into(),
            fund_id: fund_id.into(),
            borrow_amount: None,
        }
    }

    /// How much to borrow, in raw units of the fund's asset.
    pub fn amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.borrow_amount = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .borrow_amount
            .ok_or(SwaplockApiError::MissingTransferField {
                field: "borrow_amount",
            })?;
        let operation = Operation::samet_fund_borrow(SametFundBorrowOperation {
            fee: core_fee(),
            borrower: AccountId(self.borrower),
            fund_id: SametFundId(self.fund_id),
            borrow_amount: Asset::new(amount, AssetId(asset)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `samet_fund_repay`: repay a flash loan plus its fund fee.
///
/// Required: the `account`, the `fund_id`, the `.repay(..)` amount and the `.fund_fee(..)` owed.
/// Pair this with `samet_fund_borrow` in the same transaction.
pub struct SametFundRepayRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    fund_id: String,
    repay_amount: Option<(i64, String)>,
    fund_fee: Option<(i64, String)>,
}

impl<'session> SametFundRepayRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        fund_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            fund_id: fund_id.into(),
            repay_amount: None,
            fund_fee: None,
        }
    }

    /// How much principal to repay, in raw units of `asset`.
    pub fn repay(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.repay_amount = Some((amount, asset.into()));
        self
    }

    /// The fund fee owed on this loan, in raw units of `asset`.
    pub fn fund_fee(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.fund_fee = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (repay_amount, repay_asset) =
            self.repay_amount
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "repay_amount",
                })?;
        let (fee_amount, fee_asset) = self
            .fund_fee
            .ok_or(SwaplockApiError::MissingTransferField { field: "fund_fee" })?;
        let operation = Operation::samet_fund_repay(SametFundRepayOperation {
            fee: core_fee(),
            account: AccountId(self.account),
            fund_id: SametFundId(self.fund_id),
            repay_amount: Asset::new(repay_amount, AssetId(repay_asset)),
            fund_fee: Asset::new(fee_amount, AssetId(fee_asset)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
