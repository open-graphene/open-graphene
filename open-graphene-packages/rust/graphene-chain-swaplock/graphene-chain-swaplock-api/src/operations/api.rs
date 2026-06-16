use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::limit_order::{
    LimitOrderCancelRequest, LimitOrderCreateRequest, LimitOrderUpdateRequest,
};
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

    pub async fn sign_transfer_with_wif(
        self,
        prepared: PreparedTransfer,
        wif: &str,
    ) -> Result<SignedTransfer, SwaplockApiError> {
        sign_transfer_with_wif(self.session, prepared, wif).await
    }
}
