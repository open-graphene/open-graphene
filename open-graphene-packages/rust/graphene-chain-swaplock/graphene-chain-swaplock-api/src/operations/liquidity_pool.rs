//! Ergonomic builders for the liquidity pool lifecycle: create and delete.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). `create` opens a
//! pool for an asset pair backed by a share asset; `delete` closes an empty pool. Deposit, withdraw
//! and exchange live alongside once a pool has balances.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, LiquidityPoolId};
use graphene_chain_swaplock_bindings::generated::operations::{
    LiquidityPoolCreateOperation, LiquidityPoolDeleteOperation, LiquidityPoolDepositOperation,
    LiquidityPoolExchangeOperation, LiquidityPoolUpdateOperation, LiquidityPoolWithdrawOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::Asset;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const FEE_ASSET_ID: &str = "1.3.0";

/// Builder for `liquidity_pool_create`: open a pool for `asset_a`/`asset_b` backed by `share_asset`.
///
/// `asset_a` must order before `asset_b` (lower instance), and `share_asset` must be an empty user
/// asset you issue, dedicated to this pool.
pub struct LiquidityPoolCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    asset_a: Option<String>,
    asset_b: Option<String>,
    share_asset: Option<String>,
    taker_fee_percent: u16,
    withdrawal_fee_percent: u16,
}

impl<'session> LiquidityPoolCreateRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, account: impl Into<String>) -> Self {
        Self {
            session,
            account: account.into(),
            asset_a: None,
            asset_b: None,
            share_asset: None,
            taker_fee_percent: 0,
            withdrawal_fee_percent: 0,
        }
    }

    /// The pooled pair, by asset id; `asset_a` must order before `asset_b`.
    pub fn assets(mut self, asset_a: impl Into<String>, asset_b: impl Into<String>) -> Self {
        self.asset_a = Some(asset_a.into());
        self.asset_b = Some(asset_b.into());
        self
    }

    /// The asset that represents shares in the pool (an empty user asset you issue).
    pub fn share_asset(mut self, share_asset: impl Into<String>) -> Self {
        self.share_asset = Some(share_asset.into());
        self
    }

    /// Fee taken from a trade against the pool, in hundredths of a percent.
    pub fn taker_fee_percent(mut self, taker_fee_percent: u16) -> Self {
        self.taker_fee_percent = taker_fee_percent;
        self
    }

    /// Fee taken when withdrawing from the pool, in hundredths of a percent.
    pub fn withdrawal_fee_percent(mut self, withdrawal_fee_percent: u16) -> Self {
        self.withdrawal_fee_percent = withdrawal_fee_percent;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let asset_a = self
            .asset_a
            .ok_or(SwaplockApiError::MissingTransferField { field: "assets" })?;
        let asset_b = self
            .asset_b
            .ok_or(SwaplockApiError::MissingTransferField { field: "assets" })?;
        let share_asset = self
            .share_asset
            .ok_or(SwaplockApiError::MissingTransferField {
                field: "share_asset",
            })?;

        let operation = Operation::liquidity_pool_create(LiquidityPoolCreateOperation {
            fee: Asset::new(0, AssetId(FEE_ASSET_ID.to_string())),
            account: AccountId(self.account),
            asset_a: AssetId(asset_a),
            asset_b: AssetId(asset_b),
            share_asset: AssetId(share_asset),
            taker_fee_percent: self.taker_fee_percent,
            withdrawal_fee_percent: self.withdrawal_fee_percent,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `liquidity_pool_delete`: close an empty pool you own.
pub struct LiquidityPoolDeleteRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    pool: String,
}

impl<'session> LiquidityPoolDeleteRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            pool: pool.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::liquidity_pool_delete(LiquidityPoolDeleteOperation {
            fee: Asset::new(0, AssetId(FEE_ASSET_ID.to_string())),
            account: AccountId(self.account),
            pool: LiquidityPoolId(self.pool),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `liquidity_pool_deposit`: add both assets and receive pool shares.
pub struct LiquidityPoolDepositRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    pool: String,
    amount_a: Option<(i64, String)>,
    amount_b: Option<(i64, String)>,
}

impl<'session> LiquidityPoolDepositRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            pool: pool.into(),
            amount_a: None,
            amount_b: None,
        }
    }

    /// How much of the pool's first asset to add.
    pub fn amount_a(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.amount_a = Some((amount, asset_id.into()));
        self
    }

    /// How much of the pool's second asset to add.
    pub fn amount_b(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.amount_b = Some((amount, asset_id.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount_a, asset_a) = self
            .amount_a
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount_a" })?;
        let (amount_b, asset_b) = self
            .amount_b
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount_b" })?;

        let operation = Operation::liquidity_pool_deposit(LiquidityPoolDepositOperation {
            fee: Asset::new(0, AssetId(FEE_ASSET_ID.to_string())),
            account: AccountId(self.account),
            pool: LiquidityPoolId(self.pool),
            amount_a: Asset::new(amount_a, AssetId(asset_a)),
            amount_b: Asset::new(amount_b, AssetId(asset_b)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `liquidity_pool_withdraw`: burn pool shares and receive both assets back.
pub struct LiquidityPoolWithdrawRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    pool: String,
    share_amount: Option<(i64, String)>,
}

impl<'session> LiquidityPoolWithdrawRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            pool: pool.into(),
            share_amount: None,
        }
    }

    /// How many shares to burn, of the pool's share asset.
    pub fn share_amount(mut self, amount: i64, share_asset_id: impl Into<String>) -> Self {
        self.share_amount = Some((amount, share_asset_id.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, share_asset) =
            self.share_amount
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "share_amount",
                })?;

        let operation = Operation::liquidity_pool_withdraw(LiquidityPoolWithdrawOperation {
            fee: Asset::new(0, AssetId(FEE_ASSET_ID.to_string())),
            account: AccountId(self.account),
            pool: LiquidityPoolId(self.pool),
            share_amount: Asset::new(amount, AssetId(share_asset)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `liquidity_pool_exchange`: trade against the pool.
pub struct LiquidityPoolExchangeRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    pool: String,
    amount_to_sell: Option<(i64, String)>,
    min_to_receive: Option<(i64, String)>,
}

impl<'session> LiquidityPoolExchangeRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            pool: pool.into(),
            amount_to_sell: None,
            min_to_receive: None,
        }
    }

    /// What you put into the pool: raw `amount` of asset `asset_id`.
    pub fn sell(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.amount_to_sell = Some((amount, asset_id.into()));
        self
    }

    /// The minimum you accept back: raw `amount` of asset `asset_id`.
    pub fn min_to_receive(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.min_to_receive = Some((amount, asset_id.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (sell_amount, sell_asset) = self
            .amount_to_sell
            .ok_or(SwaplockApiError::MissingTransferField { field: "sell" })?;
        let (receive_amount, receive_asset) =
            self.min_to_receive
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "min_to_receive",
                })?;

        let operation = Operation::liquidity_pool_exchange(LiquidityPoolExchangeOperation {
            fee: Asset::new(0, AssetId(FEE_ASSET_ID.to_string())),
            account: AccountId(self.account),
            pool: LiquidityPoolId(self.pool),
            amount_to_sell: Asset::new(sell_amount, AssetId(sell_asset)),
            min_to_receive: Asset::new(receive_amount, AssetId(receive_asset)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `liquidity_pool_update`: change a pool's fee settings in place.
pub struct LiquidityPoolUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    pool: String,
    taker_fee_percent: Option<u16>,
    withdrawal_fee_percent: Option<u16>,
}

impl<'session> LiquidityPoolUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            pool: pool.into(),
            taker_fee_percent: None,
            withdrawal_fee_percent: None,
        }
    }

    /// New taker fee, in hundredths of a percent.
    pub fn taker_fee_percent(mut self, taker_fee_percent: u16) -> Self {
        self.taker_fee_percent = Some(taker_fee_percent);
        self
    }

    /// New withdrawal fee, in hundredths of a percent.
    pub fn withdrawal_fee_percent(mut self, withdrawal_fee_percent: u16) -> Self {
        self.withdrawal_fee_percent = Some(withdrawal_fee_percent);
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::liquidity_pool_update(LiquidityPoolUpdateOperation {
            fee: Asset::new(0, AssetId(FEE_ASSET_ID.to_string())),
            account: AccountId(self.account),
            pool: LiquidityPoolId(self.pool),
            taker_fee_percent: self.taker_fee_percent,
            withdrawal_fee_percent: self.withdrawal_fee_percent,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
