use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::sign_transfer::sign_transfer_with_wif;
use super::transfer::{PreparedTransfer, SignedTransfer, TransferRequest};

pub struct OperationsApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl<'session> OperationsApi<'session> {
    pub fn transfer(self) -> TransferRequest<'session> {
        TransferRequest::new(self.session)
    }

    pub async fn sign_transfer_with_wif(
        self,
        prepared: PreparedTransfer,
        wif: &str,
    ) -> Result<SignedTransfer, SwaplockApiError> {
        sign_transfer_with_wif(self.session, prepared, wif).await
    }
}
