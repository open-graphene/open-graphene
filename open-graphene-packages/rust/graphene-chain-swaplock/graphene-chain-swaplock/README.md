# graphene-chain-swaplock

High-level Swaplock SDK helpers for Open Graphene.

This crate is the Swaplock-specific layer above the generated protocol bindings. It provides operation builders, JSON renderers, RPC helpers, signing helpers, and live examples that compose those pieces explicitly.

Use this crate when you want to build and submit Swaplock transactions while keeping each step visible: read chain state, build a transaction, estimate and apply fees, sign, broadcast, and optionally confirm through account history.

## Module boundaries

- `operations` builds Swaplock transactions and renders signed transactions into Graphene broadcast JSON.
- `rpc` owns the WebSocket JSON-RPC transport and API-id discovery.
- `database_api` contains read-only chain-state helpers such as account lookup, asset lookup, fee estimation, balances, head-block reads, and order observation.
- `history_api` contains account-history query, polling, and confirmation matchers for operation-specific history checks.
- `network_broadcast_api` is a thin wrapper around `broadcast_transaction`.
- `transaction` mutates local transaction values, currently for applying required fees to supported operations.
- `signing` signs a transaction with WIF and verifies that the resulting compact signature matches the expected public key.
- `broadcast` composes checked signing and network broadcast when the caller does not need to inspect the signed transaction first.
- `bindings` re-exports the generated Swaplock protocol bindings crate.

The generated bindings crate remains protocol-only. High-level SDK behavior belongs here.

## Typical live operation flow

A live operation usually follows this shape:

1. Connect with `GrapheneRpc`.
2. Resolve the database, history, or network-broadcast API ids needed by the operation.
3. Read the head block and derive a transaction header.
4. Resolve account, asset, or order ids through `database_api` helpers.
5. Build an unsigned transaction through the relevant `operations` module.
6. Estimate the required fee with `database_api::required_fee`.
7. Apply the fee locally with `transaction::set_first_operation_fee` or `transaction::apply_required_fee`.
8. Run any caller-owned policy checks, such as max-fee or balance checks.
9. Sign with `signing::sign_transaction_checked`, or use `broadcast::sign_and_broadcast_transaction` if no signed-transaction inspection is needed.
10. Render signed transaction JSON with the operation module's `signed_transaction_json` function.
11. Broadcast with `network_broadcast_api::broadcast_transaction`.
12. Optionally confirm through `history_api` or another read-only database helper.

The crate does not hide these steps behind a client object yet. That is intentional: fee policy, signing policy, broadcast policy, and confirmation policy are still explicit at the call site.

## Import skeleton

```rust
use std::time::Duration;

use graphene_chain_swaplock::database_api::{
    account_balance, active_public_key_for_account, head_block, lookup_account_id,
    required_fee,
};
use graphene_chain_swaplock::history_api::{
    history_entry_matches_transfer, wait_for_account_history_confirmation,
    AccountHistoryQuery, HistoryPollConfig, TransferConfirmationCriteria,
};
use graphene_chain_swaplock::network_broadcast_api::broadcast_transaction;
use graphene_chain_swaplock::rpc::GrapheneRpc;
use graphene_chain_swaplock::signing::sign_transaction_checked;
use graphene_chain_swaplock::transaction::set_first_operation_fee;
use graphene_chain_swaplock::transfer::{
    build_transfer_transaction, signed_transaction_json, TransferTransactionInput,
};
```

The examples show complete operation-specific usage. The import skeleton is only a map of the public helper boundary.

## What the helpers do not do

These helpers deliberately do not:

- read environment variables;
- choose accounts, recipients, assets, or order ids for the caller;
- hide WIF loading or signing policy;
- perform symbol lookup policy or chain-state validation beyond the specific helper contract;
- retry failed broadcasts;
- decide max-fee limits;
- decide whether history confirmation is required;
- add memo support to flows where the common operation input does not support memo;
- invent JSON shapes outside the generated protocol bindings and existing Graphene RPC shapes.

Keep those policies in the caller, CLI, or future client layer.

## Examples

The crate includes live examples for:

- account creation preview;
- asset creation preview;
- signed transfer preview;
- a trading scenario that creates accounts/assets, issues balances, opens an unmatched limit order, and cancels it.

The shell wrappers under the Swaplock package load local environment values when present and run the examples with explicit operation labels. They are intended for the private Swaplock testnet and should not print WIF or other secrets.

Required live environment keys are:

```text
SWAPLOCK_RPC_URL
SWAPLOCK_ACTIVE_WIF
SWAPLOCK_ACCOUNT
```

Common optional keys include:

```text
SWAPLOCK_ACTIVE_PUBLIC_KEY
SWAPLOCK_ASSET_ID
SWAPLOCK_MAX_FEE
SWAPLOCK_CONFIRM_ATTEMPTS
SWAPLOCK_CONFIRM_DELAY_MS
```

Operation-specific examples document additional environment keys in their wrapper scripts.
