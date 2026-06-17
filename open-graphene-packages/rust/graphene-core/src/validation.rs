//! Account-name validation, ported from bitsharesjs `ChainValidation`.
//!
//! These rules are the same on every Graphene chain, so they live here in core rather than in any
//! one chain's API.

/// Whether `name` is a valid Graphene account name (3 to 63 chars).
///
/// Each dot-separated label must start with a lowercase letter, hold only `a-z`, `0-9` and `-`,
/// avoid a double `-`, and end on a letter or digit. Use
/// [`is_account_name_allow_short`](crate::validation::is_account_name_allow_short) to accept names
/// under three characters (e.g. while the user is still typing).
pub fn is_account_name(name: &str) -> bool {
    is_account_name_inner(name, false)
}

/// Like [`is_account_name`], but allows names shorter than three characters.
pub fn is_account_name_allow_short(name: &str) -> bool {
    is_account_name_inner(name, true)
}

fn is_account_name_inner(name: &str, allow_too_short: bool) -> bool {
    if name.is_empty() {
        return false;
    }
    let length = name.len();
    if (!allow_too_short && length < 3) || length > 63 {
        return false;
    }
    name.split('.').all(is_valid_label)
}

fn is_valid_label(label: &str) -> bool {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let is_allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    if !label.chars().all(is_allowed) {
        return false;
    }
    if label.contains("--") {
        return false;
    }
    let last = label.chars().next_back().unwrap_or(first);
    last.is_ascii_lowercase() || last.is_ascii_digit()
}

/// Whether `name` is a "cheap" account name (no premium fee).
///
/// Cheap when it carries a digit or a `-`, or has no vowel (`a e i o u y`).
pub fn is_cheap_name(name: &str) -> bool {
    name.chars().any(|c| c.is_ascii_digit() || c == '-')
        || !name
            .chars()
            .any(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_and_dotted_names() {
        assert!(is_account_name("alice"));
        assert!(is_account_name("alice-bob"));
        assert!(is_account_name("a.b.cde"));
        assert!(is_account_name("init0"));
    }

    #[test]
    fn rejects_bad_names() {
        assert!(!is_account_name(""));
        assert!(!is_account_name("ab")); // too short
        assert!(!is_account_name("1alice")); // label must start with a letter
        assert!(!is_account_name("alice-")); // label must end alnum
        assert!(!is_account_name("al--ice")); // no double dash
        assert!(!is_account_name("Alice")); // uppercase not allowed
        assert!(!is_account_name(&"a".repeat(64))); // too long
    }

    #[test]
    fn allow_too_short_accepts_short_labels() {
        assert!(!is_account_name("ab"));
        assert!(is_account_name_allow_short("ab"));
        assert!(!is_account_name_allow_short("")); // still rejects empty
    }

    #[test]
    fn cheap_when_digit_dash_or_no_vowel() {
        assert!(is_cheap_name("init0")); // digit
        assert!(is_cheap_name("foo-bar")); // dash
        assert!(is_cheap_name("bcdfg")); // no vowel (a e i o u y)
        assert!(!is_cheap_name("alice")); // has a vowel, no digit/dash
        assert!(!is_cheap_name("crypto")); // y counts as a vowel here
    }
}
