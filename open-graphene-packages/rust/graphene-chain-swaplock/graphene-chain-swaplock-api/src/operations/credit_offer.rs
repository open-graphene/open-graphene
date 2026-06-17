//! Ergonomic builders for the credit-offer family: a lender posts an `offer` of funds against
//! `acceptable_collateral`, a borrower `accept`s it (opening a `deal`), then `repay`s or tweaks the
//! deal's auto-repay setting.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). `credit_deal_expired`
//! is a virtual op (the chain emits it when a deal lapses), so it has no builder by design.

use graphene_chain_swaplock_bindings::generated::ids::{
    AccountId, AssetId, CreditDealId, CreditOfferId,
};
use graphene_chain_swaplock_bindings::generated::operations::{
    CreditDealRepayOperation, CreditDealUpdateOperation, CreditOfferAcceptOperation,
    CreditOfferCreateOperation, CreditOfferDeleteOperation, CreditOfferUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use std::time::Duration;

use graphene_chain_swaplock_bindings::generated::types::{
    Asset, CreditOfferAcceptOperationExt, Price,
};
use open_graphene_core::expiration_from_head_time;
use open_graphene_transport::GrapheneSession;

use crate::{DatabaseApi, SwaplockApiError};

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

/// Default lifetime a deal may run before it must be repaid (thirty days).
const DEFAULT_MAX_DURATION_SECONDS: u32 = 30 * 24 * 60 * 60;

/// How far ahead to auto-disable an offer when the caller does not set a time. The chain caps this
/// at 380 days from the head block, so we sit just under that.
const DEFAULT_AUTO_DISABLE_AHEAD: Duration = Duration::from_secs(364 * 24 * 60 * 60);

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// One accepted-collateral entry: the collateral asset and the price the lender values it at.
type CollateralRate = (String, ((i64, String), (i64, String)));

fn collateral_to_pairs(rates: Vec<CollateralRate>) -> Vec<(AssetId, Price)> {
    rates
        .into_iter()
        .map(
            |(asset, ((base_amount, base_asset), (quote_amount, quote_asset)))| {
                (
                    AssetId(asset),
                    Price::new(
                        Asset::new(base_amount, AssetId(base_asset)),
                        Asset::new(quote_amount, AssetId(quote_asset)),
                    ),
                )
            },
        )
        .collect()
}

fn borrowers_to_pairs(borrowers: Vec<(String, i64)>) -> Vec<(AccountId, i64)> {
    borrowers
        .into_iter()
        .map(|(account, limit)| (AccountId(account), limit))
        .collect()
}

/// Builder for `credit_offer_create`: post funds to lend.
///
/// Required: `owner_account`, the `asset_type` to lend and the `balance` to commit. Add at least one
/// `.accept_collateral(..)` so borrowers have something to pledge; the rest take sensible defaults
/// (no fee rate, thirty-day max duration, enabled, auto-disables just under the chain's cap, open to
/// any borrower).
pub struct CreditOfferCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    owner_account: String,
    asset_type: String,
    balance: i64,
    fee_rate: u32,
    max_duration_seconds: u32,
    min_deal_amount: i64,
    enabled: bool,
    auto_disable_time: Option<String>,
    acceptable_collateral: Vec<CollateralRate>,
    acceptable_borrowers: Vec<(String, i64)>,
}

impl<'session> CreditOfferCreateRequest<'session> {
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
            max_duration_seconds: DEFAULT_MAX_DURATION_SECONDS,
            min_deal_amount: 0,
            enabled: true,
            auto_disable_time: None,
            acceptable_collateral: vec![],
            acceptable_borrowers: vec![],
        }
    }

    /// Interest rate charged on a deal, in millionths (1000000 = 100%).
    pub fn fee_rate(mut self, fee_rate: u32) -> Self {
        self.fee_rate = fee_rate;
        self
    }

    /// Longest a deal opened against this offer may run before repayment is due.
    pub fn max_duration_seconds(mut self, seconds: u32) -> Self {
        self.max_duration_seconds = seconds;
        self
    }

    /// Smallest borrow this offer will accept, in raw units of the lent asset.
    pub fn min_deal_amount(mut self, amount: i64) -> Self {
        self.min_deal_amount = amount;
        self
    }

    /// Whether the offer is open for borrowing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// When the offer disables itself automatically (defaults to just under the chain's 380-day cap
    /// ahead of the head block). Format: `YYYY-MM-DDThh:mm:ss`.
    pub fn auto_disable_time(mut self, time: impl Into<String>) -> Self {
        self.auto_disable_time = Some(time.into());
        self
    }

    /// Accept `collateral_asset` as collateral, valued at the given price. The price `base` must be
    /// the lent `asset_type` and the `quote` the collateral asset. Call once per asset you take.
    pub fn accept_collateral(
        mut self,
        collateral_asset: impl Into<String>,
        base_amount: i64,
        base_asset: impl Into<String>,
        quote_amount: i64,
        quote_asset: impl Into<String>,
    ) -> Self {
        self.acceptable_collateral.push((
            collateral_asset.into(),
            (
                (base_amount, base_asset.into()),
                (quote_amount, quote_asset.into()),
            ),
        ));
        self
    }

    /// Restrict borrowing to `account`, capped at `max_amount` (omit entirely to allow anyone).
    pub fn accept_borrower(mut self, account: impl Into<String>, max_amount: i64) -> Self {
        self.acceptable_borrowers.push((account.into(), max_amount));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let auto_disable_time = match self.auto_disable_time {
            Some(time) => time,
            None => {
                let properties = DatabaseApi {
                    session: self.session,
                }
                .get_dynamic_global_properties()
                .await?;
                expiration_from_head_time(&properties.time, DEFAULT_AUTO_DISABLE_AHEAD)?
            }
        };
        let operation = Operation::credit_offer_create(CreditOfferCreateOperation {
            fee: core_fee(),
            owner_account: AccountId(self.owner_account),
            asset_type: AssetId(self.asset_type),
            balance: self.balance,
            fee_rate: self.fee_rate,
            max_duration_seconds: self.max_duration_seconds,
            min_deal_amount: self.min_deal_amount,
            enabled: self.enabled,
            auto_disable_time,
            acceptable_collateral: collateral_to_pairs(self.acceptable_collateral),
            acceptable_borrowers: borrowers_to_pairs(self.acceptable_borrowers),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `credit_offer_update`: change a live offer in place.
///
/// Required: `owner_account` and the `offer_id`. Every other field is a partial update; only the
/// setters you call are sent. `.delta_amount(..)` tops up (positive) or withdraws (negative) the
/// committed balance.
pub struct CreditOfferUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    owner_account: String,
    offer_id: String,
    delta_amount: Option<(i64, String)>,
    fee_rate: Option<u32>,
    max_duration_seconds: Option<u32>,
    min_deal_amount: Option<i64>,
    enabled: Option<bool>,
    auto_disable_time: Option<String>,
    acceptable_collateral: Option<Vec<CollateralRate>>,
    acceptable_borrowers: Option<Vec<(String, i64)>>,
}

impl<'session> CreditOfferUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        owner_account: impl Into<String>,
        offer_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            owner_account: owner_account.into(),
            offer_id: offer_id.into(),
            delta_amount: None,
            fee_rate: None,
            max_duration_seconds: None,
            min_deal_amount: None,
            enabled: None,
            auto_disable_time: None,
            acceptable_collateral: None,
            acceptable_borrowers: None,
        }
    }

    /// Top up (positive) or withdraw (negative) committed balance, in raw units of `asset`.
    pub fn delta_amount(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.delta_amount = Some((amount, asset.into()));
        self
    }

    /// New interest rate, in millionths.
    pub fn fee_rate(mut self, fee_rate: u32) -> Self {
        self.fee_rate = Some(fee_rate);
        self
    }

    /// New maximum deal duration.
    pub fn max_duration_seconds(mut self, seconds: u32) -> Self {
        self.max_duration_seconds = Some(seconds);
        self
    }

    /// New minimum borrow.
    pub fn min_deal_amount(mut self, amount: i64) -> Self {
        self.min_deal_amount = Some(amount);
        self
    }

    /// Enable or disable the offer.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    /// New auto-disable time.
    pub fn auto_disable_time(mut self, time: impl Into<String>) -> Self {
        self.auto_disable_time = Some(time.into());
        self
    }

    /// Replace the accepted-collateral set; call once per asset you want in the new set.
    pub fn accept_collateral(
        mut self,
        collateral_asset: impl Into<String>,
        base_amount: i64,
        base_asset: impl Into<String>,
        quote_amount: i64,
        quote_asset: impl Into<String>,
    ) -> Self {
        self.acceptable_collateral
            .get_or_insert_with(Vec::new)
            .push((
                collateral_asset.into(),
                (
                    (base_amount, base_asset.into()),
                    (quote_amount, quote_asset.into()),
                ),
            ));
        self
    }

    /// Replace the borrower whitelist; call once per allowed borrower.
    pub fn accept_borrower(mut self, account: impl Into<String>, max_amount: i64) -> Self {
        self.acceptable_borrowers
            .get_or_insert_with(Vec::new)
            .push((account.into(), max_amount));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::credit_offer_update(CreditOfferUpdateOperation {
            fee: core_fee(),
            owner_account: AccountId(self.owner_account),
            offer_id: CreditOfferId(self.offer_id),
            delta_amount: self
                .delta_amount
                .map(|(amount, asset)| Asset::new(amount, AssetId(asset))),
            fee_rate: self.fee_rate,
            max_duration_seconds: self.max_duration_seconds,
            min_deal_amount: self.min_deal_amount,
            enabled: self.enabled,
            auto_disable_time: self.auto_disable_time,
            acceptable_collateral: self.acceptable_collateral.map(collateral_to_pairs),
            acceptable_borrowers: self.acceptable_borrowers.map(borrowers_to_pairs),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `credit_offer_delete`: withdraw an offer and reclaim its remaining balance.
pub struct CreditOfferDeleteRequest<'session> {
    session: &'session mut GrapheneSession,
    owner_account: String,
    offer_id: String,
}

impl<'session> CreditOfferDeleteRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        owner_account: impl Into<String>,
        offer_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            owner_account: owner_account.into(),
            offer_id: offer_id.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::credit_offer_delete(CreditOfferDeleteOperation {
            fee: core_fee(),
            owner_account: AccountId(self.owner_account),
            offer_id: CreditOfferId(self.offer_id),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `credit_offer_accept`: borrow against an offer, opening a deal.
///
/// Required: the `borrower`, the `offer_id`, the `.borrow(..)` amount and the `.collateral(..)` to
/// pledge. `.max_fee_rate(..)` and `.min_duration_seconds(..)` guard against an offer that changed
/// under you; `.auto_repay(..)` sets how the resulting deal repays.
pub struct CreditOfferAcceptRequest<'session> {
    session: &'session mut GrapheneSession,
    borrower: String,
    offer_id: String,
    borrow_amount: Option<(i64, String)>,
    collateral: Option<(i64, String)>,
    max_fee_rate: u32,
    min_duration_seconds: u32,
    auto_repay: Option<u8>,
}

impl<'session> CreditOfferAcceptRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        borrower: impl Into<String>,
        offer_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            borrower: borrower.into(),
            offer_id: offer_id.into(),
            borrow_amount: None,
            collateral: None,
            max_fee_rate: u32::MAX,
            min_duration_seconds: 0,
            auto_repay: None,
        }
    }

    /// How much to borrow, in raw units of the offer's lent asset.
    pub fn borrow(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.borrow_amount = Some((amount, asset.into()));
        self
    }

    /// How much collateral to pledge, in raw units of `asset`.
    pub fn collateral(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.collateral = Some((amount, asset.into()));
        self
    }

    /// Reject the deal if the offer's interest rate exceeds this (millionths; defaults to no cap).
    pub fn max_fee_rate(mut self, max_fee_rate: u32) -> Self {
        self.max_fee_rate = max_fee_rate;
        self
    }

    /// Reject the deal if its allowed duration is shorter than this many seconds.
    pub fn min_duration_seconds(mut self, seconds: u32) -> Self {
        self.min_duration_seconds = seconds;
        self
    }

    /// How the deal repays automatically (0 disabled, 1 from balance, 2 from collateral).
    pub fn auto_repay(mut self, auto_repay: u8) -> Self {
        self.auto_repay = Some(auto_repay);
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (borrow_amount, borrow_asset) =
            self.borrow_amount
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "borrow_amount",
                })?;
        let (collateral_amount, collateral_asset) =
            self.collateral
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "collateral",
                })?;
        let operation = Operation::credit_offer_accept(CreditOfferAcceptOperation {
            fee: core_fee(),
            borrower: AccountId(self.borrower),
            offer_id: CreditOfferId(self.offer_id),
            borrow_amount: Asset::new(borrow_amount, AssetId(borrow_asset)),
            collateral: Asset::new(collateral_amount, AssetId(collateral_asset)),
            max_fee_rate: self.max_fee_rate,
            min_duration_seconds: self.min_duration_seconds,
            extensions: CreditOfferAcceptOperationExt {
                auto_repay: self.auto_repay,
            },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `credit_deal_repay`: pay back a deal (in part or full) plus its credit fee.
///
/// Required: the `account`, the `deal_id`, the `.repay(..)` amount and the `.credit_fee(..)` owed.
pub struct CreditDealRepayRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    deal_id: String,
    repay_amount: Option<(i64, String)>,
    credit_fee: Option<(i64, String)>,
}

impl<'session> CreditDealRepayRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        deal_id: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            deal_id: deal_id.into(),
            repay_amount: None,
            credit_fee: None,
        }
    }

    /// How much principal to repay, in raw units of `asset`.
    pub fn repay(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.repay_amount = Some((amount, asset.into()));
        self
    }

    /// The credit fee owed on this repayment, in raw units of `asset`.
    pub fn credit_fee(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.credit_fee = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (repay_amount, repay_asset) =
            self.repay_amount
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "repay_amount",
                })?;
        let (fee_amount, fee_asset) =
            self.credit_fee
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "credit_fee",
                })?;
        let operation = Operation::credit_deal_repay(CreditDealRepayOperation {
            fee: core_fee(),
            account: AccountId(self.account),
            deal_id: CreditDealId(self.deal_id),
            repay_amount: Asset::new(repay_amount, AssetId(repay_asset)),
            credit_fee: Asset::new(fee_amount, AssetId(fee_asset)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `credit_deal_update`: change a deal's auto-repay mode
/// (0 disabled, 1 from balance, 2 from collateral).
pub struct CreditDealUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    deal_id: String,
    auto_repay: u8,
}

impl<'session> CreditDealUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        deal_id: impl Into<String>,
        auto_repay: u8,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            deal_id: deal_id.into(),
            auto_repay,
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::credit_deal_update(CreditDealUpdateOperation {
            fee: core_fee(),
            account: AccountId(self.account),
            deal_id: CreditDealId(self.deal_id),
            auto_repay: self.auto_repay,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
