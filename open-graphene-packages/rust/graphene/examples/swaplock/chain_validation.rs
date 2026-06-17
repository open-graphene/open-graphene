use graphene::{is_account_name, is_cheap_name};

// Validate account names locally, no node needed. Same rules as bitsharesjs ChainValidation:
// 3 to 63 chars, dot-separated labels, each label a-z start, a-z0-9- body, alnum end, no "--".
fn main() {
    for name in [
        "alice", "a.b.cde", "init0", "Alice", "ab", "al--ice", "alice-",
    ] {
        let valid = is_account_name(name);
        let cheap = valid && is_cheap_name(name);
        println!("{name:>10} -> valid: {valid:<5} cheap: {cheap}");
    }
}
