//! Naming a data room member, which may be an account or a bare public key.
//!
//! Key members exist because Graphene accounts are permanent and occupy a global namespace, so
//! creating one per ephemeral process pollutes the chain forever. A key member can be granted
//! room access and can author content cards; what it cannot do is pay a fee, which is why the
//! content card operations name their payer separately from their author.

use graphene_chain_swaplock_bindings::generated::ids::{AccountId, PUBLIC_KEY_PREFIX};
use graphene_chain_swaplock_bindings::generated::static_variants::DataRoomMemberRef;

/// Whether a string names a public key rather than an account.
///
/// The chain's own `database_api` splits the two the same way, by the address prefix, so both
/// surfaces agree on what counts as a key. Account ids (`1.2.5`) and account names - which are
/// lowercase in Graphene - can never start with the uppercase prefix.
pub fn is_public_key(value: &str) -> bool {
    value.len() > PUBLIC_KEY_PREFIX.len() && value.starts_with(PUBLIC_KEY_PREFIX)
}

/// Read a member reference from a string: a public key if it carries the chain's address
/// prefix, otherwise an account id.
///
/// Note this takes an account *id* (`1.2.5`), not a name - operations travel to the chain with
/// ids already resolved.
pub fn member_ref(value: impl Into<String>) -> DataRoomMemberRef {
    let value = value.into();
    if is_public_key(&value) {
        DataRoomMemberRef::PublicKeyType(Box::new(value))
    } else {
        DataRoomMemberRef::AccountIdType(Box::new(AccountId(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_public_keys_from_account_ids() {
        assert!(is_public_key(
            "BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV"
        ));
        assert!(!is_public_key("1.2.5"));
        assert!(!is_public_key("alice"));
        // The bare prefix carries no key
        assert!(!is_public_key("BTS"));
    }

    #[test]
    fn builds_the_matching_variant_arm() {
        assert!(matches!(
            member_ref("1.2.5"),
            DataRoomMemberRef::AccountIdType(_)
        ));
        assert!(matches!(
            member_ref("BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV"),
            DataRoomMemberRef::PublicKeyType(_)
        ));
    }
}
