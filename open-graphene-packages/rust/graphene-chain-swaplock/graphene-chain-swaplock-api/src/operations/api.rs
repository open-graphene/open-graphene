use graphene_chain_swaplock_bindings::generated::types::ChainParameters;
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::account::{
    AccountCreateRequest, AccountTransferRequest, AccountUpdateRequest, AccountUpgradeRequest,
    AccountWhitelistRequest,
};
use super::assert::AssertRequest;
use super::asset::{AssetIssueRequest, AssetReserveRequest, AssetUpdateRequest};
use super::asset_admin::{
    AssetClaimFeesRequest, AssetClaimPoolRequest, AssetCreateRequest, AssetFundFeePoolRequest,
    AssetGlobalSettleRequest, AssetPublishFeedRequest, AssetSettleRequest,
    AssetUpdateBitassetRequest, AssetUpdateFeedProducersRequest, AssetUpdateIssuerRequest,
    OverrideTransferRequest,
};
use super::balance_claim::BalanceClaimRequest;
use super::blind::{BlindTransferRequest, TransferFromBlindRequest, TransferToBlindRequest};
use super::call_order::{BidCollateralRequest, CallOrderUpdateRequest};
use super::content_card::{
    ContentCardCreateRequest, ContentCardGrantCreateRequest, ContentCardGrantRevokeRequest,
    ContentCardLinkCreateRequest, ContentCardLinkRemoveRequest, ContentCardLinkUpdateRequest,
    ContentCardRemoveRequest, ContentCardUpdateRequest,
};
use super::credit_offer::{
    CreditDealRepayRequest, CreditDealUpdateRequest, CreditOfferAcceptRequest,
    CreditOfferCreateRequest, CreditOfferDeleteRequest, CreditOfferUpdateRequest,
};
use super::custom_authority::{
    CustomAuthorityCreateRequest, CustomAuthorityDeleteRequest, CustomAuthorityUpdateRequest,
};
use super::data_room::{
    DataRoomCreateRequest, DataRoomDeleteRequest, DataRoomMemberAddRequest,
    DataRoomMemberRemoveRequest, DataRoomMemberUpdateRequest, DataRoomRotateKeyRequest,
    DataRoomUpdateRequest,
};
use super::governance::{
    CommitteeMemberCreateRequest, CommitteeMemberUpdateGlobalParametersRequest,
    CommitteeMemberUpdateRequest, CustomRequest, WitnessCreateRequest, WitnessUpdateRequest,
    WorkerCreateRequest,
};
use super::htlc::{HtlcCreateRequest, HtlcExtendRequest, HtlcRedeemRequest};
use super::limit_order::{
    LimitOrderCancelRequest, LimitOrderCreateRequest, LimitOrderUpdateRequest,
};
use super::liquidity_pool::{
    LiquidityPoolCreateRequest, LiquidityPoolDeleteRequest, LiquidityPoolDepositRequest,
    LiquidityPoolExchangeRequest, LiquidityPoolUpdateRequest, LiquidityPoolWithdrawRequest,
};
use super::proposal::{ProposalCreateRequest, ProposalDeleteRequest, ProposalUpdateRequest};
use super::samet_fund::{
    SametFundBorrowRequest, SametFundCreateRequest, SametFundDeleteRequest, SametFundRepayRequest,
    SametFundUpdateRequest,
};
use super::sign_transfer::sign_transfer_with_wif;
use super::ticket::{TicketCreateRequest, TicketUpdateRequest};
use super::transaction::TransactionBuilder;
use super::transfer::{PreparedTransfer, SignedTransfer, TransferRequest};
use super::vesting::{VestingBalanceCreateRequest, VestingBalanceWithdrawRequest};
use super::withdraw_permission::{
    WithdrawPermissionClaimRequest, WithdrawPermissionCreateRequest,
    WithdrawPermissionDeleteRequest, WithdrawPermissionUpdateRequest,
};

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

    /// As the asset's issuer, forcibly move it from `from` to `to` (asset must allow override): then
    /// `.amount(..)` and `.prepare()`.
    pub fn override_transfer(
        self,
        issuer: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> OverrideTransferRequest<'session> {
        OverrideTransferRequest::new(self.session, issuer, from, to)
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

    /// Post a SameT fund for flash-loan borrowing: `owner_account` commits `balance` of `asset_type`.
    pub fn samet_fund_create(
        self,
        owner_account: impl Into<String>,
        asset_type: impl Into<String>,
        balance: i64,
    ) -> SametFundCreateRequest<'session> {
        SametFundCreateRequest::new(self.session, owner_account, asset_type, balance)
    }

    /// Change a live SameT fund in place: `owner_account`, the `fund_id`, then partial setters.
    pub fn samet_fund_update(
        self,
        owner_account: impl Into<String>,
        fund_id: impl Into<String>,
    ) -> SametFundUpdateRequest<'session> {
        SametFundUpdateRequest::new(self.session, owner_account, fund_id)
    }

    /// Withdraw a SameT fund you own, reclaiming its balance.
    pub fn samet_fund_delete(
        self,
        owner_account: impl Into<String>,
        fund_id: impl Into<String>,
    ) -> SametFundDeleteRequest<'session> {
        SametFundDeleteRequest::new(self.session, owner_account, fund_id)
    }

    /// Take a flash loan from a fund: `borrower`, the `fund_id`, then `.amount(..)`. Pair with
    /// `samet_fund_repay` in the same transaction.
    pub fn samet_fund_borrow(
        self,
        borrower: impl Into<String>,
        fund_id: impl Into<String>,
    ) -> SametFundBorrowRequest<'session> {
        SametFundBorrowRequest::new(self.session, borrower, fund_id)
    }

    /// Repay a flash loan: `account`, the `fund_id`, then `.repay(..)` and `.fund_fee(..)`.
    pub fn samet_fund_repay(
        self,
        account: impl Into<String>,
        fund_id: impl Into<String>,
    ) -> SametFundRepayRequest<'session> {
        SametFundRepayRequest::new(self.session, account, fund_id)
    }

    /// Authorise `authorized_account` to pull recurring withdrawals from `withdraw_from`; set the
    /// per-period `.limit(..)` and schedule, then `.prepare()`.
    pub fn withdraw_permission_create(
        self,
        withdraw_from: impl Into<String>,
        authorized_account: impl Into<String>,
    ) -> WithdrawPermissionCreateRequest<'session> {
        WithdrawPermissionCreateRequest::new(self.session, withdraw_from, authorized_account)
    }

    /// Change a live withdraw permission in place: `withdraw_from`, `authorized_account`, the
    /// `permission` id, then the new `.limit(..)` and schedule.
    pub fn withdraw_permission_update(
        self,
        withdraw_from: impl Into<String>,
        authorized_account: impl Into<String>,
        permission: impl Into<String>,
    ) -> WithdrawPermissionUpdateRequest<'session> {
        WithdrawPermissionUpdateRequest::new(
            self.session,
            withdraw_from,
            authorized_account,
            permission,
        )
    }

    /// Pull funds against a permission: the `permission` id, the `withdraw_from` and `withdraw_to`
    /// accounts, then `.amount(..)`.
    pub fn withdraw_permission_claim(
        self,
        permission: impl Into<String>,
        withdraw_from: impl Into<String>,
        withdraw_to: impl Into<String>,
    ) -> WithdrawPermissionClaimRequest<'session> {
        WithdrawPermissionClaimRequest::new(self.session, permission, withdraw_from, withdraw_to)
    }

    /// Revoke a withdraw permission: `withdraw_from`, `authorized_account`, the `permission` id.
    pub fn withdraw_permission_delete(
        self,
        withdraw_from: impl Into<String>,
        authorized_account: impl Into<String>,
        permission: impl Into<String>,
    ) -> WithdrawPermissionDeleteRequest<'session> {
        WithdrawPermissionDeleteRequest::new(
            self.session,
            withdraw_from,
            authorized_account,
            permission,
        )
    }

    /// Lock funds under a vesting policy: `creator`, `owner`, then `.amount(..)` and an optional
    /// `.linear(..)`/`.cdd(..)` policy (defaults to instant).
    pub fn vesting_balance_create(
        self,
        creator: impl Into<String>,
        owner: impl Into<String>,
    ) -> VestingBalanceCreateRequest<'session> {
        VestingBalanceCreateRequest::new(self.session, creator, owner)
    }

    /// Withdraw vested funds: the `vesting_balance` id, the `owner`, then `.amount(..)`.
    pub fn vesting_balance_withdraw(
        self,
        vesting_balance: impl Into<String>,
        owner: impl Into<String>,
    ) -> VestingBalanceWithdrawRequest<'session> {
        VestingBalanceWithdrawRequest::new(self.session, vesting_balance, owner)
    }

    /// Lock funds into a stake ticket tier: `account`, the `target_type` tier, then `.amount(..)`.
    pub fn ticket_create(
        self,
        account: impl Into<String>,
        target_type: u32,
    ) -> TicketCreateRequest<'session> {
        TicketCreateRequest::new(self.session, account, target_type)
    }

    /// Move a stake ticket to a different tier: the `ticket` id, the `account`, the new `target_type`.
    pub fn ticket_update(
        self,
        ticket: impl Into<String>,
        account: impl Into<String>,
        target_type: u32,
    ) -> TicketUpdateRequest<'session> {
        TicketUpdateRequest::new(self.session, ticket, account, target_type)
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

    /// Extend a hashed time-locked contract's deadline: `update_issuer` adds time via `.extend_by(..)`.
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

    /// Approve or reject a live proposal: `fee_paying_account`, the `proposal` id, then the
    /// `.approve_*`/`.unapprove_*` setters. This is how a multisig party signs off.
    pub fn proposal_update(
        self,
        fee_paying_account: impl Into<String>,
        proposal: impl Into<String>,
    ) -> ProposalUpdateRequest<'session> {
        ProposalUpdateRequest::new(self.session, fee_paying_account, proposal)
    }

    /// Drop a proposal before it executes: `fee_paying_account`, the `proposal` id.
    pub fn proposal_delete(
        self,
        fee_paying_account: impl Into<String>,
        proposal: impl Into<String>,
    ) -> ProposalDeleteRequest<'session> {
        ProposalDeleteRequest::new(self.session, fee_paying_account, proposal)
    }

    /// Register `account` as a committee member; add a page with `.url(..)`.
    pub fn committee_member_create(
        self,
        account: impl Into<String>,
    ) -> CommitteeMemberCreateRequest<'session> {
        CommitteeMemberCreateRequest::new(self.session, account)
    }

    /// Change a committee member's page: the `committee_member` id, its `account`, then `.url(..)`.
    pub fn committee_member_update(
        self,
        committee_member: impl Into<String>,
        account: impl Into<String>,
    ) -> CommitteeMemberUpdateRequest<'session> {
        CommitteeMemberUpdateRequest::new(self.session, committee_member, account)
    }

    /// Register `account` as a witness with a `block_signing_key`; add a page with `.url(..)`.
    pub fn witness_create(
        self,
        account: impl Into<String>,
        block_signing_key: impl Into<String>,
    ) -> WitnessCreateRequest<'session> {
        WitnessCreateRequest::new(self.session, account, block_signing_key)
    }

    /// Change a witness's page or signing key: the `witness` id, its `account`, then the setters.
    pub fn witness_update(
        self,
        witness: impl Into<String>,
        account: impl Into<String>,
    ) -> WitnessUpdateRequest<'session> {
        WitnessUpdateRequest::new(self.session, witness, account)
    }

    /// File a worker proposal: `owner`, a `name`, then `.daily_pay(..)`, `.work_period(..)` and an
    /// optional payout policy (`.vesting(..)`/`.burn()`, defaults to refund).
    pub fn worker_create(
        self,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> WorkerCreateRequest<'session> {
        WorkerCreateRequest::new(self.session, owner, name)
    }

    /// Post a custom-payload operation: `payer`, then `.data(..)`, `.require_auth(..)`, `.id(..)`.
    pub fn custom(self, payer: impl Into<String>) -> CustomRequest<'session> {
        CustomRequest::new(self.session, payer)
    }

    /// Delegate signing rights for `operation_type` from `account` to an authority: then
    /// `.auth_key(..)`, optional validity window and `.restriction(..)`s.
    pub fn custom_authority_create(
        self,
        account: impl Into<String>,
        operation_type: u32,
    ) -> CustomAuthorityCreateRequest<'session> {
        CustomAuthorityCreateRequest::new(self.session, account, operation_type)
    }

    /// Change a live custom authority in place: `account`, the `authority` id, then partial setters.
    pub fn custom_authority_update(
        self,
        account: impl Into<String>,
        authority: impl Into<String>,
    ) -> CustomAuthorityUpdateRequest<'session> {
        CustomAuthorityUpdateRequest::new(self.session, account, authority)
    }

    /// Drop a custom authority: `account`, the `authority` id.
    pub fn custom_authority_delete(
        self,
        account: impl Into<String>,
        authority: impl Into<String>,
    ) -> CustomAuthorityDeleteRequest<'session> {
        CustomAuthorityDeleteRequest::new(self.session, account, authority)
    }

    /// Assert that on-chain predicates hold for the transaction to apply: `fee_paying_account`, then
    /// `.assert_*(..)` predicates and optional `.require_auth(..)`.
    pub fn assert(self, fee_paying_account: impl Into<String>) -> AssertRequest<'session> {
        AssertRequest::new(self.session, fee_paying_account)
    }

    /// Claim a genesis/imported balance into `deposit_to`: the `balance` id, the `owner_key`, then
    /// `.amount(..)`.
    pub fn balance_claim(
        self,
        deposit_to: impl Into<String>,
        balance: impl Into<String>,
        owner_key: impl Into<String>,
    ) -> BalanceClaimRequest<'session> {
        BalanceClaimRequest::new(self.session, deposit_to, balance, owner_key)
    }

    /// Bid on the collateral of a globally-settled market-pegged asset: `bidder`, then
    /// `.collateral(..)` and `.debt_covered(..)`.
    pub fn bid_collateral(self, bidder: impl Into<String>) -> BidCollateralRequest<'session> {
        BidCollateralRequest::new(self.session, bidder)
    }

    /// Propose new chain-wide parameters (council-only). Pass a full [`ChainParameters`]; fetch the
    /// current ones, change what you need, and submit (usually wrapped in a proposal).
    pub fn committee_member_update_global_parameters(
        self,
        new_parameters: ChainParameters,
    ) -> CommitteeMemberUpdateGlobalParametersRequest<'session> {
        CommitteeMemberUpdateGlobalParametersRequest::new(self.session, new_parameters)
    }

    /// Move a public `amount` from `from` into blind outputs. Build the commitments, range proofs and
    /// blinding factor yourself (via [`CryptoApi`](crate::CryptoApi)); set `.blinding_factor(..)` and
    /// add `.output(..)`s so they balance the amount.
    pub fn transfer_to_blind(
        self,
        from: impl Into<String>,
        amount: i64,
        asset: impl Into<String>,
    ) -> TransferToBlindRequest<'session> {
        TransferToBlindRequest::new(self.session, from, amount, asset)
    }

    /// Move funds between blind commitments: add `.input(..)`s and `.output(..)`s that balance
    /// (inputs plus fee equal outputs).
    pub fn blind_transfer(self) -> BlindTransferRequest<'session> {
        BlindTransferRequest::new(self.session)
    }

    /// Move a public `amount` out of blind inputs to `to`. Set `.blinding_factor(..)` and add the
    /// `.input(..)`s that balance the amount plus fee.
    pub fn transfer_from_blind(
        self,
        to: impl Into<String>,
        amount: i64,
        asset: impl Into<String>,
    ) -> TransferFromBlindRequest<'session> {
        TransferFromBlindRequest::new(self.session, to, amount, asset)
    }

    /// Open a data room owned by `owner`, named `name` (unique per owner): optionally attach it via
    /// `.subject_asset(..)`/`.subject_account(..)` and set `.room_key(..)` for an encrypted room.
    pub fn data_room_create(
        self,
        owner: impl Into<String>,
        name: impl Into<String>,
    ) -> DataRoomCreateRequest<'session> {
        DataRoomCreateRequest::new(self.session, owner, name)
    }

    /// Change a data room in place: `caller`, the `room` id, then partial setters (`.new_name(..)`,
    /// `.new_subject_*(..)`, `.new_owner(..)`).
    pub fn data_room_update(
        self,
        caller: impl Into<String>,
        room: impl Into<String>,
    ) -> DataRoomUpdateRequest<'session> {
        DataRoomUpdateRequest::new(self.session, caller, room)
    }

    /// Delete a data room you own, cascade-removing its content cards, members and key epochs.
    pub fn data_room_delete(
        self,
        caller: impl Into<String>,
        room: impl Into<String>,
    ) -> DataRoomDeleteRequest<'session> {
        DataRoomDeleteRequest::new(self.session, caller, room)
    }

    /// Add `account` as a member of `room`: set `.member_key(..)` for encrypted rooms, historical
    /// `.epoch_key(..)`s, and grant `.permissions(..)` (see the `DATA_ROOM_PERM_*` flags).
    pub fn data_room_member_add(
        self,
        caller: impl Into<String>,
        room: impl Into<String>,
        account: impl Into<String>,
    ) -> DataRoomMemberAddRequest<'session> {
        DataRoomMemberAddRequest::new(self.session, caller, room, account)
    }

    /// Replace a room member's permission flags (see the `DATA_ROOM_PERM_*` constants).
    pub fn data_room_member_update(
        self,
        caller: impl Into<String>,
        room: impl Into<String>,
        account: impl Into<String>,
        permissions: u32,
    ) -> DataRoomMemberUpdateRequest<'session> {
        DataRoomMemberUpdateRequest::new(self.session, caller, room, account, permissions)
    }

    /// Remove a member from a data room (the owner cannot be removed).
    pub fn data_room_member_remove(
        self,
        caller: impl Into<String>,
        room: impl Into<String>,
        account: impl Into<String>,
    ) -> DataRoomMemberRemoveRequest<'session> {
        DataRoomMemberRemoveRequest::new(self.session, caller, room, account)
    }

    /// Rotate a room's encryption key: set `.new_room_key(..)` and a `.member_key(..)` per member.
    pub fn data_room_rotate_key(
        self,
        caller: impl Into<String>,
        room: impl Into<String>,
    ) -> DataRoomRotateKeyRequest<'session> {
        DataRoomRotateKeyRequest::new(self.session, caller, room)
    }

    /// Publish a content card in a data room: `author`, the `room`, then `.hash(..)`, `.url(..)`
    /// and `.storage_data(..)` (plus optional type, description and content key).
    pub fn content_card_create(
        self,
        author: impl Into<String>,
        room: impl Into<String>,
    ) -> ContentCardCreateRequest<'session> {
        ContentCardCreateRequest::new(self.session, author, room)
    }

    /// Change a content card in place: `caller`, the `content_id`, then partial `.new_*(..)` setters.
    pub fn content_card_update(
        self,
        caller: impl Into<String>,
        content_id: impl Into<String>,
    ) -> ContentCardUpdateRequest<'session> {
        ContentCardUpdateRequest::new(self.session, caller, content_id)
    }

    /// Remove a content card: `caller`, the `content_id`.
    pub fn content_card_remove(
        self,
        caller: impl Into<String>,
        content_id: impl Into<String>,
    ) -> ContentCardRemoveRequest<'session> {
        ContentCardRemoveRequest::new(self.session, caller, content_id)
    }

    /// Grant one card to one recipient outside room membership: `granter`, the `content_id`,
    /// then `.grantee(..)` and `.key(..)` (the card's content key encrypted to the grantee).
    pub fn content_card_grant_create(
        self,
        granter: impl Into<String>,
        content_id: impl Into<String>,
    ) -> ContentCardGrantCreateRequest<'session> {
        ContentCardGrantCreateRequest::new(self.session, granter, content_id)
    }

    /// Retract a grant: `caller`, the `grant_id`.
    pub fn content_card_grant_revoke(
        self,
        caller: impl Into<String>,
        grant_id: impl Into<String>,
    ) -> ContentCardGrantRevokeRequest<'session> {
        ContentCardGrantRevokeRequest::new(self.session, caller, grant_id)
    }

    /// Embed a card into another room by reference: `caller`, the `content_id`, the target
    /// `room`, then `.link_key(..)` for an encrypted card.
    pub fn content_card_link_create(
        self,
        caller: impl Into<String>,
        content_id: impl Into<String>,
        room: impl Into<String>,
    ) -> ContentCardLinkCreateRequest<'session> {
        ContentCardLinkCreateRequest::new(self.session, caller, content_id, room)
    }

    /// Re-wrap a link after a rotation: `caller`, the `link_id`, then `.new_link_key(..)`.
    pub fn content_card_link_update(
        self,
        caller: impl Into<String>,
        link_id: impl Into<String>,
    ) -> ContentCardLinkUpdateRequest<'session> {
        ContentCardLinkUpdateRequest::new(self.session, caller, link_id)
    }

    /// Take a card off a room's table: `caller`, the `link_id`.
    pub fn content_card_link_remove(
        self,
        caller: impl Into<String>,
        link_id: impl Into<String>,
    ) -> ContentCardLinkRemoveRequest<'session> {
        ContentCardLinkRemoveRequest::new(self.session, caller, link_id)
    }

    pub async fn sign_transfer_with_wif(
        self,
        prepared: PreparedTransfer,
        wif: &str,
    ) -> Result<SignedTransfer, SwaplockApiError> {
        sign_transfer_with_wif(self.session, prepared, wif).await
    }
}
