//! High-level Swaplock SDK facade.
//!
//! This crate is the public entry point for Swaplock-specific SDK helpers. The
//! low-level `graphene-chain-swaplock-bindings` crate still owns generated
//! protocol types and chain-local adapter implementations; this crate exposes
//! those operation modules from the chain package boundary so callers do not
//! need to depend on the bindings crate directly for normal SDK use.

pub mod account_create {
    pub use graphene_chain_swaplock_bindings::sdk::account_create::*;
}

pub mod asset_create {
    pub use graphene_chain_swaplock_bindings::sdk::asset_create::*;
}

pub mod asset_issue {
    pub use graphene_chain_swaplock_bindings::sdk::asset_issue::*;
}

pub mod limit_order_cancel {
    pub use graphene_chain_swaplock_bindings::sdk::limit_order_cancel::*;
}

pub mod limit_order_create {
    pub use graphene_chain_swaplock_bindings::sdk::limit_order_create::*;
}

pub mod transfer {
    pub use graphene_chain_swaplock_bindings::sdk::transfer::*;
}

pub use graphene_chain_swaplock_bindings as bindings;
