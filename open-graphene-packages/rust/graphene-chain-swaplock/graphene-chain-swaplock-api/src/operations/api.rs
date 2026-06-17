use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::account::{
    AccountCreateRequest, AccountTransferRequest, AccountUpdateRequest, AccountUpgradeRequest,
    AccountWhitelistRequest,
};
use super::asset::{AssetIssueRequest, AssetReserveRequest, AssetUpdateRequest};
use super::asset_admin::{
    AssetClaimFeesRequest, AssetClaimPoolRequest, AssetCreateRequest, AssetFundFeePoolRequest,
    AssetGlobalSettleRequest, AssetPublishFeedRequest, AssetSettleRequest,
    AssetUpdateBitassetRequest, AssetUpdateFeedProducersRequest, AssetUpdateIssuerRequest,
};
use super::call_order::CallOrderUpdateRequest;
use super::credit_offer::{
    CreditDealRepayRequest, CreditDealUpdateRequest, CreditOfferAcceptRequest,
    CreditOfferCreateRequest, CreditOfferDeleteRequest, CreditOfferUpdateRequest,
};
use super::htlc::{HtlcCreateRequest, HtlcExtendRequest, HtlcRedeemRequest};
use super::limit_order::{
    LimitOrderCancelRequest, LimitOrderCreateRequest, LimitOrderUpdateRequest,
};
use super::liquidity_pool::{
    LiquidityPoolCreateRequest, LiquidityPoolDeleteRequest, LiquidityPoolDepositRequest,
    LiquidityPoolExchangeRequest, LiquidityPoolUpdateRequest, LiquidityPoolWithdrawRequest,
};
use super::proposal::ProposalCreateRequest;
use super::sign_transfer::sign_transfer_with_wif;
use super::transaction::TransactionBuilder;
use super::transfer::{PreparedTransfer, SignedTransfer, TransferRequest};

pub struct OperationsApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl<'session> OperationsApi<'session> {
    pub fn transfer(self) -> TransferRequest<'session> {
        TransferRequest::new(self.session)
    }

    /// Build a signed transaction carrying any supported operation (priced, signed, ready to broadcast).
    pub fn transaction(self) -> TransactionBuilder<'session> {
        TransactionBuilder::new(self.session)
    }

    /// Place a limit order on the market: `seller` offers `.sell(..)` for at least `.receive(..)`.
    pub fn limit_order_create(
        self,
        seller: impl Into<String>,
    ) -> LimitOrderCreateRequest<'session> {
        LimitOrderCreateRequest::new(self.session, seller)
    }

    /// Change a resting limit order in place: `seller` owns it, `order` is its id. Set the moves
    /// you want with the builder (reprice, resize, extend) before `.prepare()`.
    pub fn limit_order_update(
        self,
        seller: impl Into<String>,
        order: impl Into<String>,
    ) -> LimitOrderUpdateRequest<'session> {
        LimitOrderUpdateRequest::new(self.session, seller, order)
    }

    /// Cancel a resting limit order by its id, paid for by `fee_paying_account`.
    pub fn limit_order_cancel(
        self,
        fee_paying_account: impl Into<String>,
        order: impl Into<String>,
    ) -> LimitOrderCancelRequest<'session> {
        LimitOrderCancelRequest::new(self.session, fee_paying_account, order)
    }

    /// Update an account's voting options in place: `account` is the id, then set the fields to
    /// change (memo key, voting proxy, witness/committee counts, votes) before `.prepare()`.
    pub fn account_update(self, account: impl Into<String>) -> AccountUpdateRequest<'session> {
        AccountUpdateRequest::new(self.session, account)
    }

    /// Register a new account: `registrar` pays, `name` is the new account, then `.keys(pubkey)`.
    pub fn account_create(
        self,
        registrar: impl Into<String>,
        name: impl Into<String>,
    ) -> AccountCreateRequest<'session> {
        AccountCreateRequest::new(self.session, registrar, name)
    }

    /// Upgrade an account to lifetime membership.
    pub fn account_upgrade(self, account: impl Into<String>) -> AccountUpgradeRequest<'session> {
        AccountUpgradeRequest::new(self.session, account)
    }

    /// White/black-list an account: `authorizing_account` lists `account_to_list`.
    pub fn account_whitelist(
        self,
        authorizing_account: impl Into<String>,
        account_to_list: impl Into<String>,
    ) -> AccountWhitelistRequest<'session> {
        AccountWhitelistRequest::new(self.session, authorizing_account, account_to_list)
    }

    /// Hand an account over to a new owner account (gives away control).
    pub fn account_transfer(
        self,
        account: impl Into<String>,
        new_owner: impl Into<String>,
    ) -> AccountTransferRequest<'session> {
        AccountTransferRequest::new(self.session, account, new_owner)
    }

    /// Mint units of a user asset you issue and send them to an account: `issuer` is you, then
    /// `.issue(amount, asset_id).to(account)`.
    pub fn asset_issue(self, issuer: impl Into<String>) -> AssetIssueRequest<'session> {
        AssetIssueRequest::new(self.session, issuer)
    }

    /// Burn units you hold back out of the supply: `payer` is the holder, then `.amount(..)`.
    pub fn asset_reserve(self, payer: impl Into<String>) -> AssetReserveRequest<'session> {
        AssetReserveRequest::new(self.session, payer)
    }

    /// Change a user asset's options: `issuer` owns it, `asset` is its id, then set the fields to
    /// change (description, max supply, fees, flags, new issuer) before `.prepare()`.
    pub fn asset_update(
        self,
        issuer: impl Into<String>,
        asset: impl Into<String>,
    ) -> AssetUpdateRequest<'session> {
        AssetUpdateRequest::new(self.session, issuer, asset)
    }

    /// Create a new user-issued asset: `issuer`, `symbol`, `precision`, then optional setters.
    pub fn asset_create(
        self,
        issuer: impl Into<String>,
        symbol: impl Into<String>,
        precision: u8,
    ) -> AssetCreateRequest<'session> {
        AssetCreateRequest::new(self.session, issuer, symbol, precision)
    }

    /// Hand a user asset to a new issuer.
    pub fn asset_update_issuer(
        self,
        issuer: impl Into<String>,
        asset: impl Into<String>,
        new_issuer: impl Into<String>,
    ) -> AssetUpdateIssuerRequest<'session> {
        AssetUpdateIssuerRequest::new(self.session, issuer, asset, new_issuer)
    }

    /// Top up an asset's fee pool with `amount` of core asset.
    pub fn asset_fund_fee_pool(
        self,
        from_account: impl Into<String>,
        asset: impl Into<String>,
        amount: i64,
    ) -> AssetFundFeePoolRequest<'session> {
        AssetFundFeePoolRequest::new(self.session, from_account, asset, amount)
    }

    /// Reclaim `amount` of core asset from an asset's fee pool.
    pub fn asset_claim_pool(
        self,
        issuer: impl Into<String>,
        asset: impl Into<String>,
        amount: i64,
    ) -> AssetClaimPoolRequest<'session> {
        AssetClaimPoolRequest::new(self.session, issuer, asset, amount)
    }

    /// Collect accumulated market fees: `issuer`, then `.amount(..)`.
    pub fn asset_claim_fees(self, issuer: impl Into<String>) -> AssetClaimFeesRequest<'session> {
        AssetClaimFeesRequest::new(self.session, issuer)
    }

    /// Settle a market-pegged asset for collateral: `account`, then `.amount(..)`.
    pub fn asset_settle(self, account: impl Into<String>) -> AssetSettleRequest<'session> {
        AssetSettleRequest::new(self.session, account)
    }

    /// Globally settle a market-pegged asset: `issuer`, `asset`, then `.settle_price(..)`.
    pub fn asset_global_settle(
        self,
        issuer: impl Into<String>,
        asset: impl Into<String>,
    ) -> AssetGlobalSettleRequest<'session> {
        AssetGlobalSettleRequest::new(self.session, issuer, asset)
    }

    /// Set the feed producers for a market-pegged asset: `issuer`, `asset`, then `.producers(..)`.
    pub fn asset_update_feed_producers(
        self,
        issuer: impl Into<String>,
        asset: impl Into<String>,
    ) -> AssetUpdateFeedProducersRequest<'session> {
        AssetUpdateFeedProducersRequest::new(self.session, issuer, asset)
    }

    /// Publish a price feed: `publisher`, `asset`, then `.settlement_price(..)`/`.core_exchange_rate(..)`.
    pub fn asset_publish_feed(
        self,
        publisher: impl Into<String>,
        asset: impl Into<String>,
    ) -> AssetPublishFeedRequest<'session> {
        AssetPublishFeedRequest::new(self.session, publisher, asset)
    }

    /// Change a market-pegged asset's bitasset options: `issuer`, `asset`, `short_backing_asset`.
    pub fn asset_update_bitasset(
        self,
        issuer: impl Into<String>,
        asset: impl Into<String>,
        short_backing_asset: impl Into<String>,
    ) -> AssetUpdateBitassetRequest<'session> {
        AssetUpdateBitassetRequest::new(self.session, issuer, asset, short_backing_asset)
    }

    /// Lock funds in a hashed time-locked contract from `from` to `to`; set `.amount(..)` and
    /// `.lock_sha256(secret)`, optionally `.claim_period(..)`, then `.prepare()`.
    pub fn htlc_create(
        self,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> HtlcCreateRequest<'session> {
        HtlcCreateRequest::new(self.session, from, to)
    }

    /// Claim a hashed time-locked contract by its id, revealing the `.preimage(..)`.
    pub fn htlc_redeem(
        self,
        htlc: impl Into<String>,
        redeemer: impl Into<String>,
    ) -> HtlcRedeemRequest<'session> {
        HtlcRedeemRequest::new(self.session, htlc, redeemer)
    }

    /// Adjust a margin position: `funding_account` sets `.delta_collateral(..)` and
    /// `.delta_debt(..)` (negative to withdraw/repay) before `.prepare()`.
    pub fn call_order_update(
        self,
        funding_account: impl Into<String>,
    ) -> CallOrderUpdateRequest<'session> {
        CallOrderUpdateRequest::new(self.session, funding_account)
    }

    /// Post a credit offer to lend: `owner_account` commits `balance` of `asset_type`, then add at
    /// least one `.accept_collateral(..)` before `.prepare()`.
    pub fn credit_offer_create(
        self,
        owner_account: impl Into<String>,
        asset_type: impl Into<String>,
        balance: i64,
    ) -> CreditOfferCreateRequest<'session> {
        CreditOfferCreateRequest::new(self.session, owner_account, asset_type, balance)
    }

    /// Change a live credit offer in place: `owner_account`, the `offer_id`, then partial setters.
    pub fn credit_offer_update(
        self,
        owner_account: impl Into<String>,
        offer_id: impl Into<String>,
    ) -> CreditOfferUpdateRequest<'session> {
        CreditOfferUpdateRequest::new(self.session, owner_account, offer_id)
    }

    /// Withdraw a credit offer you own, reclaiming its remaining balance.
    pub fn credit_offer_delete(
        self,
        owner_account: impl Into<String>,
        offer_id: impl Into<String>,
    ) -> CreditOfferDeleteRequest<'session> {
        CreditOfferDeleteRequest::new(self.session, owner_account, offer_id)
    }

    /// Borrow against an offer: `borrower`, the `offer_id`, then `.borrow(..)` and `.collateral(..)`.
    pub fn credit_offer_accept(
        self,
        borrower: impl Into<String>,
        offer_id: impl Into<String>,
    ) -> CreditOfferAcceptRequest<'session> {
        CreditOfferAcceptRequest::new(self.session, borrower, offer_id)
    }

    /// Repay a credit deal: `account`, the `deal_id`, then `.repay(..)` and `.credit_fee(..)`.
    pub fn credit_deal_repay(
        self,
        account: impl Into<String>,
        deal_id: impl Into<String>,
    ) -> CreditDealRepayRequest<'session> {
        CreditDealRepayRequest::new(self.session, account, deal_id)
    }

    /// Change a credit deal's auto-repay mode (0 disabled, 1 from balance, 2 from collateral).
    pub fn credit_deal_update(
        self,
        account: impl Into<String>,
        deal_id: impl Into<String>,
        auto_repay: u8,
    ) -> CreditDealUpdateRequest<'session> {
        CreditDealUpdateRequest::new(self.session, account, deal_id, auto_repay)
    }

    /// Open a liquidity pool for an asset pair: `account` owns it, then `.assets(a, b)` and
    /// `.share_asset(id)` (an empty user asset you issue), optional fees, then `.prepare()`.
    pub fn liquidity_pool_create(
        self,
        account: impl Into<String>,
    ) -> LiquidityPoolCreateRequest<'session> {
        LiquidityPoolCreateRequest::new(self.session, account)
    }

    /// Close an empty liquidity pool you own, by its id.
    pub fn liquidity_pool_delete(
        self,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> LiquidityPoolDeleteRequest<'session> {
        LiquidityPoolDeleteRequest::new(self.session, account, pool)
    }

    /// Add both assets to a pool and receive shares: `.amount_a(..)` and `.amount_b(..)`.
    pub fn liquidity_pool_deposit(
        self,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> LiquidityPoolDepositRequest<'session> {
        LiquidityPoolDepositRequest::new(self.session, account, pool)
    }

    /// Burn pool shares to get both assets back: `.share_amount(..)`.
    pub fn liquidity_pool_withdraw(
        self,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> LiquidityPoolWithdrawRequest<'session> {
        LiquidityPoolWithdrawRequest::new(self.session, account, pool)
    }

    /// Trade against a pool: `.sell(..)` for at least `.min_to_receive(..)`.
    pub fn liquidity_pool_exchange(
        self,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> LiquidityPoolExchangeRequest<'session> {
        LiquidityPoolExchangeRequest::new(self.session, account, pool)
    }

    /// Change a pool's fees: `.taker_fee_percent(..)` / `.withdrawal_fee_percent(..)`.
    pub fn liquidity_pool_update(
        self,
        account: impl Into<String>,
        pool: impl Into<String>,
    ) -> LiquidityPoolUpdateRequest<'session> {
        LiquidityPoolUpdateRequest::new(self.session, account, pool)
    }

    /// Extend a hashed time-locked contract's deadline: `update_issuer` adds time via `.add(..)`.
    pub fn htlc_extend(
        self,
        htlc: impl Into<String>,
        update_issuer: impl Into<String>,
    ) -> HtlcExtendRequest<'session> {
        HtlcExtendRequest::new(self.session, htlc, update_issuer)
    }

    /// Wrap operations in a proposal paid for by `fee_paying_account`: add them with `.propose(..)`,
    /// set `.expiration(..)`/`.review_period(..)`, then `.prepare()`. The wrapped ops are priced.
    pub fn proposal_create(
        self,
        fee_paying_account: impl Into<String>,
    ) -> ProposalCreateRequest<'session> {
        ProposalCreateRequest::new(self.session, fee_paying_account)
    }

    pub async fn sign_transfer_with_wif(
        self,
        prepared: PreparedTransfer,
        wif: &str,
    ) -> Result<SignedTransfer, SwaplockApiError> {
        sign_transfer_with_wif(self.session, prepared, wif).await
    }
}
