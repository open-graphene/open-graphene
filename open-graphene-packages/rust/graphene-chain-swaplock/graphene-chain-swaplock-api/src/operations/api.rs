use open_graphene_transport::GrapheneSession;

use crate::{DatabaseApi, SwaplockApiError};

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
        let mut database = DatabaseApi {
            session: self.session,
        };
        let account = database.get_account_by_id(prepared.from_id()).await?;
        let expected_public_key = single_active_public_key(&account.active)?;
        prepared.sign_with_wif(wif, &expected_public_key)
    }
}

fn single_active_public_key(
    authority: &graphene_chain_swaplock_bindings::generated::Authority,
) -> Result<String, SwaplockApiError> {
    match authority.key_auths.as_slice() {
        [(public_key, weight)] if u32::from(*weight) >= authority.weight_threshold => {
            Ok(public_key.clone())
        }
        [] => Err(SwaplockApiError::InvalidTransfer {
            message: "active authority has no public keys; provide expected public key explicitly"
                .to_string(),
        }),
        [_] => Err(SwaplockApiError::InvalidTransfer {
            message: "single active public key weight is below authority threshold".to_string(),
        }),
        _ => Err(SwaplockApiError::InvalidTransfer {
            message:
                "active authority has multiple public keys; provide expected public key explicitly"
                    .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_chain_swaplock_bindings::generated::Authority;

    #[test]
    fn accepts_single_sufficient_active_public_key() {
        let authority = Authority {
            weight_threshold: 1,
            account_auths: vec![],
            key_auths: vec![("BTS-public-key".to_string(), 1)],
            address_auths: vec![],
        };

        assert_eq!(
            single_active_public_key(&authority).unwrap(),
            "BTS-public-key"
        );
    }

    #[test]
    fn rejects_missing_active_public_key() {
        let authority = Authority {
            weight_threshold: 1,
            account_auths: vec![],
            key_auths: vec![],
            address_auths: vec![],
        };

        assert_eq!(
            single_active_public_key(&authority)
                .unwrap_err()
                .to_string(),
            "invalid transfer: active authority has no public keys; provide expected public key explicitly"
        );
    }

    #[test]
    fn rejects_multiple_active_public_keys() {
        let authority = Authority {
            weight_threshold: 1,
            account_auths: vec![],
            key_auths: vec![("one".to_string(), 1), ("two".to_string(), 1)],
            address_auths: vec![],
        };

        assert_eq!(
            single_active_public_key(&authority)
                .unwrap_err()
                .to_string(),
            "invalid transfer: active authority has multiple public keys; provide expected public key explicitly"
        );
    }

    #[test]
    fn rejects_insufficient_single_active_public_key_weight() {
        let authority = Authority {
            weight_threshold: 2,
            account_auths: vec![],
            key_auths: vec![("BTS-public-key".to_string(), 1)],
            address_auths: vec![],
        };

        assert_eq!(
            single_active_public_key(&authority)
                .unwrap_err()
                .to_string(),
            "invalid transfer: single active public key weight is below authority threshold"
        );
    }
}
