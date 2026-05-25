use open_graphene_sdk_core::TransactionHeader;

use crate::builder::GrapheneOperationBuilderTypes;
use crate::common::{AccountRefInput, FeeInput, PublicKeyInput, SingleKeyAuthorityInput};

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

pub trait AccountCreateAdapter {
    type Transaction;
    type Error;

    fn build_account_create_transaction(
        input: AccountCreateInput,
    ) -> Result<Self::Transaction, Self::Error>;
}

pub trait AccountCreateChainTypes: GrapheneOperationBuilderTypes {
    type Authority;
    type AccountOptions;
    type AccountCreateOperation;
    type AccountCreateOperationExtensions;

    fn single_key_authority(public_key: String) -> Self::Authority;

    fn account_options(memo_key: String, voting_account: Self::AccountId) -> Self::AccountOptions;

    fn empty_account_create_extensions() -> Self::AccountCreateOperationExtensions;

    fn account_create_operation(
        fee: Self::Asset,
        registrar: Self::AccountId,
        referrer: Self::AccountId,
        referrer_percent: u16,
        name: String,
        owner: Self::Authority,
        active: Self::Authority,
        options: Self::AccountOptions,
        extensions: Self::AccountCreateOperationExtensions,
    ) -> Self::AccountCreateOperation;

    fn operation_account_create(operation: Self::AccountCreateOperation) -> Self::Operation;
}

pub fn build_account_create_transaction_for<C: AccountCreateChainTypes>(
    input: AccountCreateInput,
) -> C::Transaction {
    let fee = C::asset(input.fee.amount, input.fee.asset_id.into());
    let owner = C::single_key_authority(input.owner.public_key.value);
    let active = C::single_key_authority(input.active.public_key.value);
    let options = C::account_options(input.memo_key.value, input.voting_account.id.into());
    let operation = C::account_create_operation(
        fee,
        input.registrar.id.into(),
        input.referrer.id.into(),
        input.referrer_percent,
        input.name,
        owner,
        active,
        options,
        C::empty_account_create_extensions(),
    );

    C::transaction(
        input.header,
        vec![C::operation_account_create(operation)],
        C::empty_extensions(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC_KEY: &str = "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV";

    fn header() -> TransactionHeader {
        TransactionHeader {
            ref_block_num: 2,
            ref_block_prefix: 3,
            expiration: "2026-05-25T12:01:00".to_string(),
        }
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
            PUBLIC_KEY,
            PUBLIC_KEY,
            PUBLIC_KEY,
            "1.2.5",
        );

        assert_eq!(input.registrar.id, "1.2.100");
        assert_eq!(input.referrer.id, "1.2.101");
        assert_eq!(input.referrer_percent, 5_000);
        assert_eq!(input.name, "new-account");
        assert_eq!(input.owner.public_key.value, PUBLIC_KEY);
        assert_eq!(input.voting_account.id, "1.2.5");
    }
}
