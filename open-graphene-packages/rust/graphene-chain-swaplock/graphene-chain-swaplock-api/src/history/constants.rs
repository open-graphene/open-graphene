pub const DEFAULT_ACCOUNT_HISTORY_LIMIT: u32 = 20;
pub const DEFAULT_ACCOUNT_HISTORY_OFFSET: u32 = 0;
pub const MAX_ACCOUNT_HISTORY_LIMIT: u32 = 98;

pub(super) const ACCOUNT_HISTORY_METHOD: &str = "get_account_history";
pub(super) const HISTORY_START_SENTINEL: &str = "1.11.0";
pub(super) const HISTORY_STOP_SENTINEL: &str = "1.11.0";
pub(super) const ACCOUNT_HISTORY_CALLBACK_ID: u64 = 6;
