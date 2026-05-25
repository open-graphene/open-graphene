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

impl TransferInput {
    pub fn new(
        header: TransactionHeader,
        from_id: impl Into<String>,
        to_id: impl Into<String>,
        amount: i64,
        asset_id: impl Into<String>,
        fee_amount: i64,
        fee_asset_id: impl Into<String>,
    ) -> Self {
        Self {
            header,
            from: AccountRefInput::new(from_id),
            to: AccountRefInput::new(to_id),
            amount: AssetAmountInput::new(amount, asset_id),
            fee: FeeInput::new(fee_amount, fee_asset_id),
        }
    }
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

impl AccountCreateInput {
    pub fn simple(
        header: TransactionHeader,
        fee: FeeInput,
        registrar_id: impl Into<String>,
        referrer_id: impl Into<String>,
        referrer_percent: u16,
        name: impl Into<String>,
        owner_public_key: impl Into<String>,
        active_public_key: impl Into<String>,
        memo_key: impl Into<String>,
        voting_account_id: impl Into<String>,
    ) -> Self {
        Self {
            header,
            fee,
            registrar: AccountRefInput::new(registrar_id),
            referrer: AccountRefInput::new(referrer_id),
            referrer_percent,
            name: name.into(),
            owner: SingleKeyAuthorityInput::new(owner_public_key),
            active: SingleKeyAuthorityInput::new(active_public_key),
            memo_key: PublicKeyInput::new(memo_key),
            voting_account: AccountRefInput::new(voting_account_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIssueInput {
    pub header: TransactionHeader,
    pub fee: FeeInput,
    pub issuer: AccountRefInput,
    pub issue_to_account: AccountRefInput,
    pub asset_to_issue: AssetAmountInput,
}

impl AssetIssueInput {
    pub fn new(
        header: TransactionHeader,
        fee: FeeInput,
        issuer_id: impl Into<String>,
        issue_to_account_id: impl Into<String>,
        amount: i64,
        asset_id: impl Into<String>,
    ) -> Self {
        Self {
            header,
            fee,
            issuer: AccountRefInput::new(issuer_id),
            issue_to_account: AccountRefInput::new(issue_to_account_id),
            asset_to_issue: AssetAmountInput::new(amount, asset_id),
        }
    }
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

impl AssetCreateInput {
    pub fn uia(
        header: TransactionHeader,
        fee: FeeInput,
        issuer_id: impl Into<String>,
        symbol: impl Into<String>,
        precision: u8,
        max_supply: i64,
        description: impl Into<String>,
    ) -> Self {
        Self {
            header,
            fee,
            issuer: AccountRefInput::new(issuer_id),
            symbol: symbol.into(),
            precision,
            max_supply,
            description: description.into(),
        }
    }
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

    fn header() -> TransactionHeader {
        TransactionHeader {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
        }
    }

    #[test]
    fn transfer_input_constructor_wraps_raw_fields() {
        let input = TransferInput::new(
            header(),
            "1.2.100",
            "1.2.101",
            100_000,
            "1.3.1",
            200_000,
            "1.3.0",
        );

        assert_eq!(input.header.ref_block_num, 2);
        assert_eq!(input.from.id, "1.2.100");
        assert_eq!(input.to.id, "1.2.101");
        assert_eq!(input.amount, AssetAmountInput::new(100_000, "1.3.1"));
        assert_eq!(input.fee, FeeInput::new(200_000, "1.3.0"));
    }

    #[test]
    fn account_create_simple_constructor_wraps_supported_simple_shape() {
        let input = AccountCreateInput::simple(
            header(),
            FeeInput::core(500_000),
            "1.2.100",
            "1.2.101",
            5_000,
            "new-account",
            "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV",
            "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV",
            "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV",
            "1.2.5",
        );

        assert_eq!(input.registrar.id, "1.2.100");
        assert_eq!(input.referrer.id, "1.2.101");
        assert_eq!(input.referrer_percent, 5_000);
        assert_eq!(input.name, "new-account");
        assert_eq!(
            input.owner.public_key.value,
            "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV"
        );
        assert_eq!(input.voting_account.id, "1.2.5");
    }

    #[test]
    fn asset_issue_constructor_wraps_raw_issue_fields() {
        let input = AssetIssueInput::new(
            header(),
            FeeInput::new(200_000, "1.3.0"),
            "1.2.100",
            "1.2.101",
            100_000,
            "1.3.1",
        );

        assert_eq!(input.issuer.id, "1.2.100");
        assert_eq!(input.issue_to_account.id, "1.2.101");
        assert_eq!(
            input.asset_to_issue,
            AssetAmountInput::new(100_000, "1.3.1")
        );
    }

    #[test]
    fn asset_create_uia_constructor_wraps_minimal_asset_create_fields() {
        let input = AssetCreateInput::uia(
            header(),
            FeeInput::new(500_000, "1.3.0"),
            "1.2.100",
            "OGT12345",
            5,
            1_000_000_000_000,
            "open-graphene live asset_create proof",
        );

        assert_eq!(input.issuer.id, "1.2.100");
        assert_eq!(input.symbol, "OGT12345");
        assert_eq!(input.precision, 5);
        assert_eq!(input.max_supply, 1_000_000_000_000);
        assert_eq!(input.description, "open-graphene live asset_create proof");
    }
}
