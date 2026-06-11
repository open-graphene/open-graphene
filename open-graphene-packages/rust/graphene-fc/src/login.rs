//! Account login: derive an account's keys straight from its name and password.
//!
//! A wallet can log in with nothing stored, just the account name and password, by re-deriving the
//! same keys every time. Derivation matches bitsharesjs `AccountLogin`, so the keys line up with
//! any Graphene wallet for the same credentials.

use crate::PrivateKey;
use crate::brainkey::normalize_brain_key;

/// The three standard role keys an account is built from.
///
/// Anyone with the account name and password can reproduce these, so treat the password like the
/// keys themselves. Reach for `.active` to sign ordinary operations, `.owner` for account changes,
/// `.memo` for memo encryption.
pub struct AccountKeys {
    pub owner: PrivateKey,
    pub active: PrivateKey,
    pub memo: PrivateKey,
}

impl AccountKeys {
    /// Derive the owner, active and memo keys for `account_name` from `password`.
    pub fn derive(account_name: &str, password: &str) -> Self {
        Self {
            owner: account_role_key(account_name, password, "owner"),
            active: account_role_key(account_name, password, "active"),
            memo: account_role_key(account_name, password, "memo"),
        }
    }
}

/// Derive the key for one `role` (e.g. `"active"`), the way bitsharesjs does:
/// `sha256(normalize(account_name + role + password))`. Use this for non-standard roles.
pub fn account_role_key(account_name: &str, password: &str, role: &str) -> PrivateKey {
    let seed = normalize_brain_key(&format!("{account_name}{role}{password}"));
    PrivateKey::from_seed(seed.as_bytes()).expect("a sha256 digest is a valid secp256k1 scalar")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_credentials_derive_the_same_keys() {
        let a = AccountKeys::derive("alice", "correct horse battery staple");
        let b = AccountKeys::derive("alice", "correct horse battery staple");
        assert_eq!(a.active.to_wif(), b.active.to_wif());
        assert_eq!(a.owner.to_wif(), b.owner.to_wif());
        assert_eq!(a.memo.to_wif(), b.memo.to_wif());
    }

    #[test]
    fn the_three_roles_are_different_keys() {
        let keys = AccountKeys::derive("alice", "password");
        assert_ne!(keys.owner.to_wif(), keys.active.to_wif());
        assert_ne!(keys.active.to_wif(), keys.memo.to_wif());
        assert_ne!(keys.owner.to_wif(), keys.memo.to_wif());
    }

    #[test]
    fn a_different_account_or_password_changes_the_keys() {
        let base = AccountKeys::derive("alice", "password").active.to_wif();
        assert_ne!(base, AccountKeys::derive("bob", "password").active.to_wif());
        assert_ne!(base, AccountKeys::derive("alice", "other").active.to_wif());
    }

    #[test]
    fn matches_bitsharesjs_login_vector() {
        // bitsharesjs test/chain/Login.js: account "someaccountname" / "somereallylongpassword".
        // Its auths check holds this active public key (GPH prefix). Byte-identical derivation.
        let active = AccountKeys::derive("someaccountname", "somereallylongpassword").active;
        assert_eq!(
            active.to_public_key().to_prefixed_string("GPH"),
            "GPH5Abm5dCdy3hJ1C5ckXkqUH2Me7dXqi9Y7yjn9ACaiSJ9h8r8mL"
        );
    }

    #[test]
    fn whitespace_in_the_password_is_normalised() {
        let tidy = account_role_key("alice", "two words", "active").to_wif();
        let messy = account_role_key("alice", "two   words", "active").to_wif();
        assert_eq!(tidy, messy);
    }
}
