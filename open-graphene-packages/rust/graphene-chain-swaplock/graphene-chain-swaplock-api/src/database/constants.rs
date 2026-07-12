//! Fixed callback ids for session-mode subscriptions. They live in the
//! transport's reserved subscription range so they can never collide with
//! request/broadcast-callback ids (see SUBSCRIPTION_CALLBACK_ID_BASE).

use open_graphene_transport::SUBSCRIPTION_CALLBACK_ID_BASE;

pub(crate) const ACCOUNT_BALANCES_CALLBACK_ID: u64 = SUBSCRIPTION_CALLBACK_ID_BASE + 4;
pub(crate) const ACCOUNT_ORDERS_CALLBACK_ID: u64 = SUBSCRIPTION_CALLBACK_ID_BASE + 5;
pub(crate) const ACCOUNT_CALLBACK_ID: u64 = SUBSCRIPTION_CALLBACK_ID_BASE + 2;
pub(crate) const ASSET_CALLBACK_ID: u64 = SUBSCRIPTION_CALLBACK_ID_BASE + 3;
pub(crate) const DYNAMIC_GLOBAL_PROPERTIES_ID: &str = "2.1.0";
pub(crate) const DYNAMIC_GLOBAL_PROPERTIES_CALLBACK_ID: u64 = SUBSCRIPTION_CALLBACK_ID_BASE + 1;
