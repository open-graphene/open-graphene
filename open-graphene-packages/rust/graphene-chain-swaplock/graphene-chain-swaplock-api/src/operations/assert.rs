//! Ergonomic builder for `assert`: bundle predicates that must hold for the transaction to apply.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). Each `.assert_*`
//! setter adds a predicate; the chain rejects the transaction if any fails to hold when applied.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId};
use graphene_chain_swaplock_bindings::generated::operations::AssertOperation;
use graphene_chain_swaplock_bindings::generated::static_variants::{Operation, Predicate};
use graphene_chain_swaplock_bindings::generated::types::{
    AccountNameEqLitPredicate, Asset, AssetSymbolEqLitPredicate, BlockIdPredicate,
};
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Builder for `assert`: `fee_paying_account` asserts that every added predicate holds.
///
/// Required: the `fee_paying_account` and at least one predicate. Add `.require_auth(..)` for each
/// account whose authority must sign.
pub struct AssertRequest<'session> {
    session: &'session mut GrapheneSession,
    fee_paying_account: String,
    predicates: Vec<Predicate>,
    required_auths: Vec<String>,
}

impl<'session> AssertRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        fee_paying_account: impl Into<String>,
    ) -> Self {
        Self {
            session,
            fee_paying_account: fee_paying_account.into(),
            predicates: vec![],
            required_auths: vec![],
        }
    }

    /// Assert that account `account_id` still has the name `name`.
    pub fn assert_account_name(
        mut self,
        account_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        self.predicates
            .push(Predicate::AccountNameEqLitPredicate(Box::new(
                AccountNameEqLitPredicate {
                    account_id: AccountId(account_id.into()),
                    name: name.into(),
                },
            )));
        self
    }

    /// Assert that asset `asset_id` still has the symbol `symbol`.
    pub fn assert_asset_symbol(
        mut self,
        asset_id: impl Into<String>,
        symbol: impl Into<String>,
    ) -> Self {
        self.predicates
            .push(Predicate::AssetSymbolEqLitPredicate(Box::new(
                AssetSymbolEqLitPredicate {
                    asset_id: AssetId(asset_id.into()),
                    symbol: symbol.into(),
                },
            )));
        self
    }

    /// Assert that a recent block has the given 20-byte id (a soft fork guard).
    pub fn assert_block_id(mut self, block_id: impl Into<Vec<u8>>) -> Self {
        self.predicates
            .push(Predicate::BlockIdPredicate(Box::new(BlockIdPredicate {
                id: block_id.into(),
            })));
        self
    }

    /// An account whose authority must sign the assertion.
    pub fn require_auth(mut self, account: impl Into<String>) -> Self {
        self.required_auths.push(account.into());
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        if self.predicates.is_empty() {
            return Err(SwaplockApiError::MissingTransferField { field: "predicate" });
        }
        let operation = Operation::assert(AssertOperation {
            fee: core_fee(),
            fee_paying_account: AccountId(self.fee_paying_account),
            predicates: self.predicates,
            required_auths: self.required_auths.into_iter().map(AccountId).collect(),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
