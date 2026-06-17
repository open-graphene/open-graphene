use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::account::AccountUpdateRequest;
use super::asset::{AssetIssueRequest, AssetReserveRequest, AssetUpdateRequest};
use super::htlc::{HtlcCreateRequest, HtlcRedeemRequest};
use super::limit_order::{
    LimitOrderCancelRequest, LimitOrderCreateRequest, LimitOrderUpdateRequest,
};
use super::liquidity_pool::{LiquidityPoolCreateRequest, LiquidityPoolDeleteRequest};
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
