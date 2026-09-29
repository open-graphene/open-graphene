//! Ergonomic builder for `call_order_update`: adjust a margin position's collateral and debt.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). The debt asset is
//! a market-pegged asset; positive deltas add collateral / borrow more, negative deltas remove
//! collateral / repay.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_swaplock_bindings::generated::operations::{
    BidCollateralOperation, CallOrderUpdateOperation,
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

/// Builder for `call_order_update`: `funding_account` changes its margin position by the deltas.
pub struct CallOrderUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    funding_account: String,
    delta_collateral: Option<(i64, String)>,
    delta_debt: Option<(i64, String)>,
}

impl<'session> CallOrderUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        funding_account: impl Into<String>,
    ) -> Self {
        Self {
            session,
            funding_account: funding_account.into(),
            delta_collateral: None,
            delta_debt: None,
        }
    }

    /// Change collateral by `amount` (negative to withdraw) of the backing asset `asset_id`.
    pub fn delta_collateral(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.delta_collateral = Some((amount, asset_id.into()));
        self
    }

    /// Change debt by `amount` (negative to repay) of the market-pegged asset `asset_id`.
    pub fn delta_debt(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.delta_debt = Some((amount, asset_id.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (collateral_amount, collateral_asset) =
            self.delta_collateral
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "delta_collateral",
                })?;
        let (debt_amount, debt_asset) =
            self.delta_debt
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "delta_debt",
                })?;

        let operation = Operation::call_order_update(CallOrderUpdateOperation {
            fee: core_fee(),
            funding_account: AccountId(self.funding_account),
            delta_collateral: Asset::new(collateral_amount, AssetId(collateral_asset)),
            delta_debt: Asset::new(debt_amount, AssetId(debt_asset)),
            extensions: graphene_chain_swaplock_bindings::generated::types::CallOrderUpdateOperationOptionsType { target_collateral_ratio: None },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `bid_collateral`: bid on the collateral of a globally-settled market-pegged asset.
///
/// Required: the `bidder`, the `.collateral(..)` to lock and the `.debt_covered(..)` to take on. The
/// bid competes for the settlement fund; it executes when the asset is revived.
pub struct BidCollateralRequest<'session> {
    session: &'session mut GrapheneSession,
    bidder: String,
    collateral: Option<(i64, String)>,
    debt_covered: Option<(i64, String)>,
}

impl<'session> BidCollateralRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, bidder: impl Into<String>) -> Self {
        Self {
            session,
            bidder: bidder.into(),
            collateral: None,
            debt_covered: None,
        }
    }

    /// Additional collateral to lock behind the bid, in raw units of the backing asset.
    pub fn collateral(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.collateral = Some((amount, asset.into()));
        self
    }

    /// Debt to cover with the bid, in raw units of the market-pegged asset.
    pub fn debt_covered(mut self, amount: i64, asset: impl Into<String>) -> Self {
        self.debt_covered = Some((amount, asset.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (collateral_amount, collateral_asset) =
            self.collateral
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "collateral",
                })?;
        let (debt_amount, debt_asset) =
            self.debt_covered
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "debt_covered",
                })?;
        let operation = Operation::bid_collateral(BidCollateralOperation {
            fee: core_fee(),
            bidder: AccountId(self.bidder),
            additional_collateral: Asset::new(collateral_amount, AssetId(collateral_asset)),
            debt_covered: Asset::new(debt_amount, AssetId(debt_asset)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
