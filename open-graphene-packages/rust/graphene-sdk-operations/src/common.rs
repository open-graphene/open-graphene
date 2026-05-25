use open_graphene_sdk_primitives::{AccountIdRef, AssetIdRef, LimitOrderIdRef, ObjectIdError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeInput {
    pub amount: i64,
    pub asset_id: String,
}

impl FeeInput {
    pub fn new(amount: i64, asset_id: impl Into<String>) -> Self {
        Self {
            amount,
            asset_id: asset_id.into(),
        }
    }

    pub fn checked(amount: i64, asset_id: impl Into<String>) -> Result<Self, ObjectIdError> {
        let asset_id = asset_id.into();
        AssetIdRef::parse(&asset_id)?;
        Ok(Self { amount, asset_id })
    }

    pub fn core(amount: i64) -> Self {
        Self::new(amount, "1.3.0")
    }

    pub fn asset_id_ref(&self) -> Result<AssetIdRef, ObjectIdError> {
        AssetIdRef::parse(&self.asset_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetAmountInput {
    pub amount: i64,
    pub asset_id: String,
}

impl AssetAmountInput {
    pub fn new(amount: i64, asset_id: impl Into<String>) -> Self {
        Self {
            amount,
            asset_id: asset_id.into(),
        }
    }

    pub fn checked(amount: i64, asset_id: impl Into<String>) -> Result<Self, ObjectIdError> {
        let asset_id = asset_id.into();
        AssetIdRef::parse(&asset_id)?;
        Ok(Self { amount, asset_id })
    }

    pub fn asset_id_ref(&self) -> Result<AssetIdRef, ObjectIdError> {
        AssetIdRef::parse(&self.asset_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountRefInput {
    pub id: String,
}

impl AccountRefInput {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn checked(id: impl Into<String>) -> Result<Self, ObjectIdError> {
        let id = id.into();
        AccountIdRef::parse(&id)?;
        Ok(Self { id })
    }

    pub fn account_id_ref(&self) -> Result<AccountIdRef, ObjectIdError> {
        AccountIdRef::parse(&self.id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitOrderRefInput {
    pub id: String,
}

impl LimitOrderRefInput {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn checked(id: impl Into<String>) -> Result<Self, ObjectIdError> {
        let id = id.into();
        LimitOrderIdRef::parse(&id)?;
        Ok(Self { id })
    }

    pub fn limit_order_id_ref(&self) -> Result<LimitOrderIdRef, ObjectIdError> {
        LimitOrderIdRef::parse(&self.id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKeyInput {
    pub value: String,
}

impl PublicKeyInput {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleKeyAuthorityInput {
    pub public_key: PublicKeyInput,
}

impl SingleKeyAuthorityInput {
    pub fn new(public_key: impl Into<String>) -> Self {
        Self {
            public_key: PublicKeyInput::new(public_key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use open_graphene_sdk_primitives::ObjectIdError;

    #[test]
    fn fee_input_defaults_to_core_asset() {
        let fee = FeeInput::core(42);

        assert_eq!(fee.amount, 42);
        assert_eq!(fee.asset_id, "1.3.0");
        assert_eq!(fee.asset_id_ref().unwrap().to_string(), "1.3.0");
    }

    #[test]
    fn fee_input_checked_accepts_asset_ids() {
        let fee = FeeInput::checked(42, "1.3.0").unwrap();

        assert_eq!(fee.amount, 42);
        assert_eq!(fee.asset_id, "1.3.0");
    }

    #[test]
    fn fee_input_checked_rejects_non_asset_ids() {
        assert!(matches!(
            FeeInput::checked(42, "1.2.100"),
            Err(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn account_refs_validate_account_ids() {
        assert_eq!(
            AccountRefInput::new("1.2.100")
                .account_id_ref()
                .unwrap()
                .to_string(),
            "1.2.100"
        );
        assert!(matches!(
            AccountRefInput::new("1.3.0").account_id_ref(),
            Err(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn account_ref_checked_accepts_account_ids() {
        let account = AccountRefInput::checked("1.2.100").unwrap();

        assert_eq!(account.id, "1.2.100");
    }

    #[test]
    fn account_ref_checked_rejects_non_account_ids() {
        assert!(matches!(
            AccountRefInput::checked("1.3.0"),
            Err(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn limit_order_refs_validate_limit_order_ids() {
        assert_eq!(
            LimitOrderRefInput::new("1.7.123")
                .limit_order_id_ref()
                .unwrap()
                .to_string(),
            "1.7.123"
        );
        assert!(matches!(
            LimitOrderRefInput::new("1.2.100").limit_order_id_ref(),
            Err(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn limit_order_ref_checked_accepts_limit_order_ids() {
        let order = LimitOrderRefInput::checked("1.7.123").unwrap();

        assert_eq!(order.id, "1.7.123");
    }

    #[test]
    fn limit_order_ref_checked_rejects_non_limit_order_ids() {
        assert!(matches!(
            LimitOrderRefInput::checked("1.2.100"),
            Err(ObjectIdError::UnexpectedType { .. })
        ));
    }

    #[test]
    fn asset_amount_checked_accepts_asset_ids() {
        let amount = AssetAmountInput::checked(100_000, "1.3.1").unwrap();

        assert_eq!(amount.amount, 100_000);
        assert_eq!(amount.asset_id, "1.3.1");
    }

    #[test]
    fn asset_amount_checked_rejects_non_asset_ids() {
        assert!(matches!(
            AssetAmountInput::checked(1, "1.2.100"),
            Err(ObjectIdError::UnexpectedType { .. })
        ));
    }
}
