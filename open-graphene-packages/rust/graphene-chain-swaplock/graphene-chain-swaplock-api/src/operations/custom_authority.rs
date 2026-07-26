//! Ergonomic builders for custom authorities: an account delegates the right to sign a specific
//! operation type to another authority, optionally constrained by argument restrictions.
//!
//! Thin sugar over [`TransactionBuilder`](super::transaction::TransactionBuilder). The common case
//! (delegate an operation type to one key, unconstrained) needs only `.auth_key(..)`. Field-level
//! constraints are advanced: build [`Restriction`]s yourself (with [`ArgumentType`]) and add them.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, AssetId, CustomAuthorityId};
use graphene_chain_swaplock_bindings::generated::operations::{
    CustomAuthorityCreateOperation, CustomAuthorityDeleteOperation, CustomAuthorityUpdateOperation,
};
use graphene_chain_swaplock_bindings::generated::static_variants::Operation;
use graphene_chain_swaplock_bindings::generated::types::{Asset, Authority, Restriction};
use open_graphene_core::expiration_from_head_time;
use open_graphene_transport::GrapheneSession;

use std::time::Duration;

use crate::{DatabaseApi, SwaplockApiError};

use super::account::single_key_authority;
use super::transaction::{PreparedTransaction, TransactionBuilder};

const CORE_ASSET_ID: &str = "1.3.0";

/// How long a custom authority stays valid when the caller does not set a window (one year).
const DEFAULT_VALID_AHEAD: Duration = Duration::from_secs(365 * 24 * 60 * 60);

fn core_fee() -> Asset {
    Asset::new(0, AssetId(CORE_ASSET_ID.to_string()))
}

/// Builder for `custom_authority_create`: grant `operation_type` signing rights to an authority.
///
/// Required: the `account` granting the right, the `operation_type` (the numeric tag of the
/// operation being delegated) and the `.auth_key(..)` allowed to sign. The validity window defaults
/// to one year from the head block; add `.restriction(..)`s to constrain the operation's fields.
pub struct CustomAuthorityCreateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    operation_type: u32,
    enabled: bool,
    valid_from: Option<String>,
    valid_to: Option<String>,
    auth: Option<Authority>,
    restrictions: Vec<Restriction>,
}

impl<'session> CustomAuthorityCreateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        operation_type: u32,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            operation_type,
            enabled: true,
            valid_from: None,
            valid_to: None,
            auth: None,
            restrictions: vec![],
        }
    }

    /// Allow `key` (single-key authority) to sign the delegated operation.
    pub fn auth_key(mut self, key: impl Into<String>) -> Self {
        self.auth = Some(single_key_authority(key.into()));
        self
    }

    /// Set the full authority allowed to sign, instead of a single key.
    pub fn auth(mut self, auth: Authority) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Whether the authority is active (default true).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Validity window (defaults to head-block time .. one year later). Format: `YYYY-MM-DDThh:mm:ss`.
    pub fn valid_period(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.valid_from = Some(from.into());
        self.valid_to = Some(to.into());
        self
    }

    /// Constrain a field of the delegated operation. Advanced; build the [`Restriction`] yourself.
    pub fn restriction(mut self, restriction: Restriction) -> Self {
        self.restrictions.push(restriction);
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let auth = self
            .auth
            .ok_or(SwaplockApiError::MissingTransferField { field: "auth" })?;
        let (valid_from, valid_to) = match (self.valid_from, self.valid_to) {
            (Some(from), Some(to)) => (from, to),
            _ => {
                let head = DatabaseApi {
                    session: self.session,
                }
                .get_dynamic_global_properties()
                .await?
                .time;
                let valid_to = expiration_from_head_time(&head, DEFAULT_VALID_AHEAD)?;
                (head, valid_to)
            }
        };
        let operation = Operation::custom_authority_create(CustomAuthorityCreateOperation {
            fee: core_fee(),
            account: AccountId(self.account),
            enabled: self.enabled,
            valid_from,
            valid_to,
            operation_type: u64::from(self.operation_type),
            auth,
            restrictions: self.restrictions,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `custom_authority_update`: change a live custom authority in place.
///
/// Required: the `account` and the `authority` id. Every other field is a partial update; only the
/// setters you call are sent. `.add_restriction(..)`/`.remove_restriction(..)` edit the constraints.
pub struct CustomAuthorityUpdateRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    authority: String,
    new_enabled: Option<bool>,
    new_valid_from: Option<String>,
    new_valid_to: Option<String>,
    new_auth: Option<Authority>,
    restrictions_to_remove: Vec<u16>,
    restrictions_to_add: Vec<Restriction>,
}

impl<'session> CustomAuthorityUpdateRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        authority: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            authority: authority.into(),
            new_enabled: None,
            new_valid_from: None,
            new_valid_to: None,
            new_auth: None,
            restrictions_to_remove: vec![],
            restrictions_to_add: vec![],
        }
    }

    /// Enable or disable the authority.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.new_enabled = Some(enabled);
        self
    }

    /// New validity window. Format: `YYYY-MM-DDThh:mm:ss`.
    pub fn valid_period(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.new_valid_from = Some(from.into());
        self.new_valid_to = Some(to.into());
        self
    }

    /// Replace the authority allowed to sign with a single key.
    pub fn auth_key(mut self, key: impl Into<String>) -> Self {
        self.new_auth = Some(single_key_authority(key.into()));
        self
    }

    /// Replace the authority allowed to sign.
    pub fn auth(mut self, auth: Authority) -> Self {
        self.new_auth = Some(auth);
        self
    }

    /// Remove an existing restriction by its index.
    pub fn remove_restriction(mut self, index: u16) -> Self {
        self.restrictions_to_remove.push(index);
        self
    }

    /// Add a restriction. Advanced; build the [`Restriction`] yourself.
    pub fn add_restriction(mut self, restriction: Restriction) -> Self {
        self.restrictions_to_add.push(restriction);
        self
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::custom_authority_update(CustomAuthorityUpdateOperation {
            fee: core_fee(),
            account: AccountId(self.account),
            authority_to_update: CustomAuthorityId(self.authority),
            new_enabled: self.new_enabled,
            new_valid_from: self.new_valid_from,
            new_valid_to: self.new_valid_to,
            new_auth: self.new_auth,
            restrictions_to_remove: self.restrictions_to_remove,
            restrictions_to_add: self.restrictions_to_add,
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}

/// Builder for `custom_authority_delete`: drop a custom authority.
pub struct CustomAuthorityDeleteRequest<'session> {
    session: &'session mut GrapheneSession,
    account: String,
    authority: String,
}

impl<'session> CustomAuthorityDeleteRequest<'session> {
    pub(super) fn new(
        session: &'session mut GrapheneSession,
        account: impl Into<String>,
        authority: impl Into<String>,
    ) -> Self {
        Self {
            session,
            account: account.into(),
            authority: authority.into(),
        }
    }

    pub async fn prepare(self) -> Result<PreparedTransaction, SwaplockApiError> {
        let operation = Operation::custom_authority_delete(CustomAuthorityDeleteOperation {
            fee: core_fee(),
            account: AccountId(self.account),
            authority_to_delete: CustomAuthorityId(self.authority),
            extensions: vec![],
        });
        TransactionBuilder::new(self.session)
            .add_operation(operation)
            .prepare()
            .await
    }
}
