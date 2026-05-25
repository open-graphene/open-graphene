use crate::{AccountIdRef, AssetIdRef, ObjectIdError, TransactionHeader};

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

    pub fn account_id_ref(&self) -> Result<AccountIdRef, ObjectIdError> {
        AccountIdRef::parse(&self.id)
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferInput {
    pub header: TransactionHeader,
    pub from: AccountRefInput,
    pub to: AccountRefInput,
    pub amount: AssetAmountInput,
    pub fee: FeeInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub registrar: AccountRefInput,
    pub referrer: AccountRefInput,
    pub referrer_percent: u16,
    pub name: String,
    pub owner: SingleKeyAuthorityInput,
    pub active: SingleKeyAuthorityInput,
    pub memo_key: PublicKeyInput,
    pub voting_account: AccountRefInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIssueInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub issuer: AccountRefInput,
    pub issue_to_account: AccountRefInput,
    pub asset_to_issue: AssetAmountInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetCreateInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub issuer: AccountRefInput,
    pub symbol: String,
    pub precision: u8,
    pub max_supply: i64,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_input_defaults_to_core_asset() {
        let fee = FeeInput::core(42);

        assert_eq!(fee.amount, 42);
        assert_eq!(fee.asset_id, "1.3.0");
        assert_eq!(fee.asset_id_ref().unwrap().to_string(), "1.3.0");
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
    fn asset_amount_refs_validate_asset_ids() {
        let amount = AssetAmountInput::new(100_000, "1.3.1");

        assert_eq!(amount.asset_id_ref().unwrap().to_string(), "1.3.1");
        assert!(matches!(
            AssetAmountInput::new(1, "1.2.100").asset_id_ref(),
            Err(ObjectIdError::UnexpectedType { .. })
        ));
    }
}
