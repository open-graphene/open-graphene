//! Ergonomic builders for the rest of the asset operations: lifecycle (`create`, `update_issuer`),
//! fee pool (`fund_fee_pool`, `claim_pool`, `claim_fees`) and the market-pegged-asset family
//! (`settle`, `global_settle`, `update_feed_producers`, `publish_feed`, `update_bitasset`).
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). The MPA family is
//! only meaningful on a chain with bitassets; on swaplock the node parses and prices these ops but
//! has no market-pegged asset to apply them to.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_swaplock_bindings::generated::operations::{
    AssetClaimFeesOperation, AssetClaimPoolOperation, AssetCreateOperation,
    AssetFundFeePoolOperation, AssetGlobalSettleOperation, AssetPublishFeedOperation,
    AssetSettleOperation, AssetUpdateBitassetOperation, AssetUpdateFeedProducersOperation,
    AssetUpdateIssuerOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{
    AdditionalAssetOptions, Asset, AssetClaimFeesOperationAdditionalOptionsType, AssetOptions,
    AssetPublishFeedOperationExt, BitassetOptions, BitassetOptionsExt, Price, PriceFeed,
};
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Builder for `asset_create`: register a new user-issued asset.
///
/// Sensible defaults (no market fee, permissive supply, 1:1 core exchange rate); set the few fields
/// you care about. Bitasset (market-pegged) creation is out of scope here.
pub struct AssetCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    symbol: String,
    precision: u8,
    max_supply: i64,
    market_fee_percent: u16,
    description: String,
}

impl<'session> AssetCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        issuer: impl Into<String>,
        symbol: impl Into<String>,
        precision: u8,
    ) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            symbol: symbol.into(),
            precision,
            max_supply: 1_000_000_000_000_000,
            market_fee_percent: 0,
            description: String::new(),
        }
    }

    /// Cap on total supply, in raw units (default very large).
    pub fn max_supply(mut self, max_supply: i64) -> Self {
        self.max_supply = max_supply;
        self
    }

    /// Market fee in hundredths of a percent.
    pub fn market_fee_percent(mut self, market_fee_percent: u16) -> Self {
        self.market_fee_percent = market_fee_percent;
        self
    }

    /// Free-text description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let core = || Asset::new(1, AssetId(CORE_ASSET_ID.to_string()));
        let operation = Operation::asset_create(AssetCreateOperation {
            fee: core_fee(),
            issuer: AccountId(self.issuer),
            symbol: self.symbol,
            precision: self.precision,
            common_options: AssetOptions {
                max_supply: self.max_supply,
                market_fee_percent: self.market_fee_percent,
                max_market_fee: 0,
                issuer_permissions: 0,
                flags: 0,
                core_exchange_rate: Price::new(core(), core()),
                whitelist_authorities: vec![],
                blacklist_authorities: vec![],
                whitelist_markets: vec![],
                blacklist_markets: vec![],
                description: self.description,
                extensions: AdditionalAssetOptions {
                    reward_percent: None,
                    whitelist_market_fee_sharing: None,
                    taker_fee_percent: None,
                },
            },
            bitasset_opts: None,
            is_prediction_market: false,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_update_issuer`: hand a user asset to a new issuer.
pub struct AssetUpdateIssuerRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    asset: String,
    new_issuer: String,
}

impl<'session> AssetUpdateIssuerRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        issuer: impl Into<String>,
        asset: impl Into<String>,
        new_issuer: impl Into<String>,
    ) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            asset: asset.into(),
            new_issuer: new_issuer.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::asset_update_issuer(AssetUpdateIssuerOperation {
            fee: core_fee(),
            issuer: AccountId(self.issuer),
            asset_to_update: AssetId(self.asset),
            new_issuer: AccountId(self.new_issuer),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_fund_fee_pool`: top up an asset's fee pool with core asset.
pub struct AssetFundFeePoolRequest<'session> {
    session: &'session mut GrapheneSession,
    from_account: String,
    asset: String,
    amount: i64,
}

impl<'session> AssetFundFeePoolRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        from_account: impl Into<String>,
        asset: impl Into<String>,
        amount: i64,
    ) -> Self {
        Self {
            session,
            from_account: from_account.into(),
            asset: asset.into(),
            amount,
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::asset_fund_fee_pool(AssetFundFeePoolOperation {
            fee: core_fee(),
            from_account: AccountId(self.from_account),
            asset_id: AssetId(self.asset),
            amount: self.amount,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_claim_pool`: reclaim core asset from an asset's fee pool.
pub struct AssetClaimPoolRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    asset: String,
    amount: i64,
}

impl<'session> AssetClaimPoolRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        issuer: impl Into<String>,
        asset: impl Into<String>,
        amount: i64,
    ) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            asset: asset.into(),
            amount,
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::asset_claim_pool(AssetClaimPoolOperation {
            fee: core_fee(),
            issuer: AccountId(self.issuer),
            asset_id: AssetId(self.asset),
            amount_to_claim: Asset::new(self.amount, AssetId(CORE_ASSET_ID.to_string())),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_claim_fees`: the issuer collects accumulated market fees.
pub struct AssetClaimFeesRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    amount: Option<(i64, String)>,
}

impl<'session> AssetClaimFeesRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, issuer: impl Into<String>) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            amount: None,
        }
    }

    /// How much to claim: raw `amount` of asset `asset_id`.
    pub fn amount(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.amount = Some((amount, asset_id.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let operation = Operation::asset_claim_fees(AssetClaimFeesOperation {
            fee: core_fee(),
            issuer: AccountId(self.issuer),
            amount_to_claim: Asset::new(amount, AssetId(asset)),
            extensions: AssetClaimFeesOperationAdditionalOptionsType {
                claim_from_asset_id: None,
            },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_settle`: settle a market-pegged asset for its backing collateral.
pub struct AssetSettleRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    amount: Option<(i64, String)>,
}

impl<'session> AssetSettleRequest<'session> {
    pub(super) fn new(session: &'session mut GrapheneSession, account: impl Into<String>) -> Self {
        Self {
            session,
            account: account.into(),
            amount: None,
        }
    }

    /// How much to settle: raw `amount` of the market-pegged asset `asset_id`.
    pub fn amount(mut self, amount: i64, asset_id: impl Into<String>) -> Self {
        self.amount = Some((amount, asset_id.into()));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let (amount, asset) = self
            .amount
            .ok_or(SwaplockApiError::MissingTransferField { field: "amount" })?;
        let operation = Operation::asset_settle(AssetSettleOperation {
            fee: core_fee(),
            account: AccountId(self.account),
            amount: Asset::new(amount, AssetId(asset)),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_global_settle`: the issuer settles a whole market-pegged asset at a price.
pub struct AssetGlobalSettleRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    asset: String,
    settle_price: Option<((i64, String), (i64, String))>,
}

impl<'session> AssetGlobalSettleRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        issuer: impl Into<String>,
        asset: impl Into<String>,
    ) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            asset: asset.into(),
            settle_price: None,
        }
    }

    /// The settlement price as `base` per `quote`, each `(amount, asset_id)`.
    pub fn settle_price(
        mut self,
        base_amount: i64,
        base_asset: impl Into<String>,
        quote_amount: i64,
        quote_asset: impl Into<String>,
    ) -> Self {
        self.settle_price = Some((
            (base_amount, base_asset.into()),
            (quote_amount, quote_asset.into()),
        ));
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let ((base_amount, base_asset), (quote_amount, quote_asset)) =
            self.settle_price
                .ok_or(SwaplockApiError::MissingTransferField {
                    field: "settle_price",
                })?;
        let operation = Operation::asset_global_settle(AssetGlobalSettleOperation {
            fee: core_fee(),
            issuer: AccountId(self.issuer),
            asset_to_settle: AssetId(self.asset),
            settle_price: Price::new(
                Asset::new(base_amount, AssetId(base_asset)),
                Asset::new(quote_amount, AssetId(quote_asset)),
            ),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_update_feed_producers`: set who may publish price feeds for an asset.
pub struct AssetUpdateFeedProducersRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    asset: String,
    producers: Vec<String>,
}

impl<'session> AssetUpdateFeedProducersRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        issuer: impl Into<String>,
        asset: impl Into<String>,
    ) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            asset: asset.into(),
            producers: vec![],
        }
    }

    /// The set of accounts allowed to publish feeds.
    pub fn producers<I, S>(mut self, producers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.producers = producers.into_iter().map(Into::into).collect();
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::asset_update_feed_producers(AssetUpdateFeedProducersOperation {
            fee: core_fee(),
            issuer: AccountId(self.issuer),
            asset_to_update: AssetId(self.asset),
            new_feed_producers: self.producers.into_iter().map(AccountId).collect(),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_publish_feed`: a producer publishes a price feed for a market-pegged asset.
pub struct AssetPublishFeedRequest<'session> {
    session: &'session mut GrapheneSession,
    publisher: String,
    asset: String,
    settlement_price: Option<((i64, String), (i64, String))>,
    core_exchange_rate: Option<((i64, String), (i64, String))>,
    maintenance_collateral_ratio: u16,
    maximum_short_squeeze_ratio: u16,
}

impl<'session> AssetPublishFeedRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        publisher: impl Into<String>,
        asset: impl Into<String>,
    ) -> Self {
        Self {
            session,
            publisher: publisher.into(),
            asset: asset.into(),
            settlement_price: None,
            core_exchange_rate: None,
            maintenance_collateral_ratio: 1750,
            maximum_short_squeeze_ratio: 1500,
        }
    }

    /// The settlement price, `base` per `quote`, each `(amount, asset_id)`.
    pub fn settlement_price(
        mut self,
        base_amount: i64,
        base_asset: impl Into<String>,
        quote_amount: i64,
        quote_asset: impl Into<String>,
    ) -> Self {
        self.settlement_price = Some((
            (base_amount, base_asset.into()),
            (quote_amount, quote_asset.into()),
        ));
        self
    }

    /// The core exchange rate, `base` per `quote`, each `(amount, asset_id)`.
    pub fn core_exchange_rate(
        mut self,
        base_amount: i64,
        base_asset: impl Into<String>,
        quote_amount: i64,
        quote_asset: impl Into<String>,
    ) -> Self {
        self.core_exchange_rate = Some((
            (base_amount, base_asset.into()),
            (quote_amount, quote_asset.into()),
        ));
        self
    }

    /// Maintenance collateral ratio in tenths of a percent (default 1750 = 175%).
    pub fn maintenance_collateral_ratio(mut self, ratio: u16) -> Self {
        self.maintenance_collateral_ratio = ratio;
        self
    }

    /// Maximum short squeeze ratio in tenths of a percent (default 1500 = 150%).
    pub fn maximum_short_squeeze_ratio(mut self, ratio: u16) -> Self {
        self.maximum_short_squeeze_ratio = ratio;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let to_price = |pair: Option<((i64, String), (i64, String))>,
                        field|
         -> Result<Price, SwaplockApiError> {
            let ((base_amount, base_asset), (quote_amount, quote_asset)) =
                pair.ok_or(SwaplockApiError::MissingTransferField { field })?;
            Ok(Price::new(
                Asset::new(base_amount, AssetId(base_asset)),
                Asset::new(quote_amount, AssetId(quote_asset)),
            ))
        };
        let settlement_price = to_price(self.settlement_price, "settlement_price")?;
        let core_exchange_rate = to_price(self.core_exchange_rate, "core_exchange_rate")?;

        let operation = Operation::asset_publish_feed(AssetPublishFeedOperation {
            fee: core_fee(),
            publisher: AccountId(self.publisher),
            asset_id: AssetId(self.asset),
            feed: PriceFeed {
                settlement_price,
                maintenance_collateral_ratio: self.maintenance_collateral_ratio,
                maximum_short_squeeze_ratio: self.maximum_short_squeeze_ratio,
                core_exchange_rate,
            },
            extensions: AssetPublishFeedOperationExt {
                initial_collateral_ratio: None,
            },
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `asset_update_bitasset`: change a market-pegged asset's bitasset options.
pub struct AssetUpdateBitassetRequest<'session> {
    session: &'session mut GrapheneSession,
    issuer: String,
    asset: String,
    short_backing_asset: String,
    feed_lifetime_sec: u32,
    minimum_feeds: u8,
    force_settlement_delay_sec: u32,
    force_settlement_offset_percent: u16,
    maximum_force_settlement_volume: u16,
}

impl<'session> AssetUpdateBitassetRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        issuer: impl Into<String>,
        asset: impl Into<String>,
        short_backing_asset: impl Into<String>,
    ) -> Self {
        Self {
            session,
            issuer: issuer.into(),
            asset: asset.into(),
            short_backing_asset: short_backing_asset.into(),
            feed_lifetime_sec: 86_400,
            minimum_feeds: 1,
            force_settlement_delay_sec: 86_400,
            force_settlement_offset_percent: 0,
            maximum_force_settlement_volume: 2_000,
        }
    }

    /// How long a feed stays valid (seconds).
    pub fn feed_lifetime_sec(mut self, value: u32) -> Self {
        self.feed_lifetime_sec = value;
        self
    }

    /// Minimum number of feeds for a median.
    pub fn minimum_feeds(mut self, value: u8) -> Self {
        self.minimum_feeds = value;
        self
    }

    /// Delay before a forced settlement executes (seconds).
    pub fn force_settlement_delay_sec(mut self, value: u32) -> Self {
        self.force_settlement_delay_sec = value;
        self
    }

    /// Forced settlement price offset, in hundredths of a percent.
    pub fn force_settlement_offset_percent(mut self, value: u16) -> Self {
        self.force_settlement_offset_percent = value;
        self
    }

    /// Cap on forced settlement volume per maintenance interval, in hundredths of a percent.
    pub fn maximum_force_settlement_volume(mut self, value: u16) -> Self {
        self.maximum_force_settlement_volume = value;
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::asset_update_bitasset(AssetUpdateBitassetOperation {
            fee: core_fee(),
            issuer: AccountId(self.issuer),
            asset_to_update: AssetId(self.asset),
            new_options: BitassetOptions {
                feed_lifetime_sec: self.feed_lifetime_sec,
                minimum_feeds: self.minimum_feeds,
                force_settlement_delay_sec: self.force_settlement_delay_sec,
                force_settlement_offset_percent: self.force_settlement_offset_percent,
                maximum_force_settlement_volume: self.maximum_force_settlement_volume,
                short_backing_asset: AssetId(self.short_backing_asset),
                extensions: BitassetOptionsExt {
                    initial_collateral_ratio: None,
                },
            },
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
