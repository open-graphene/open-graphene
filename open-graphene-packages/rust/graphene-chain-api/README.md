# graphene-chain-api

The high-level Rust API of Open Graphene, shared by every chain: sessions, database, history,
orders and crypto queries, live subscriptions, operation builders, transaction preparation,
signing and broadcast.

This directory is not a crate of its own. Each chain has a thin crate that builds this source
against its generated bindings:

| Crate | Bindings | Features |
|---|---|---|
| `graphene-chain-actanet-api` | `graphene-chain-actanet-bindings` | `bitshares7` |
| `graphene-chain-bitshares-api` | `graphene-chain-bitshares-bindings` | `bitshares7` |
| `graphene-chain-rsquared-api` | `graphene-chain-rsquared-bindings` | none |

A chain crate points its `[lib] path` at `src/lib.rs` here and depends on its bindings under the
name `bindings`:

```toml
[lib]
path = "../../graphene-chain-api/src/lib.rs"

[dependencies]
bindings = { package = "graphene-chain-actanet-bindings", path = "../graphene-chain-actanet-bindings" }
```

The generated types have the same names on every Graphene chain, but they are distinct types in
each bindings crate, so the source is compiled once per chain rather than made generic over them.

## The `bitshares7` feature

Turns on what BitShares 7 has and older Graphene chains such as R-Squared lack: blind transfers,
collateral bids and call order options, credit offers and deals, limit order updates and options,
liquidity pools, samet funds, and the witness signature in `get_block_header`. A chain crate
enables it when its bindings have these operations.

Chain constants come from the bindings: `CHAIN_NAME`, `CHAIN_ID` (the genesis hash) and
`PUBLIC_KEY_PREFIX`. Code and tests in this source must not assume one chain's prefix, operation
numbering or chain id.

## Networks

One chain has several networks (mainnet, testnet, a local node), each with its own genesis and so
its own chain id. `CHAIN_ID` is only the network the bindings were generated against. A prepared
transaction or transfer signs for the chain id the node reported when it was prepared
(`PreparedTransaction::chain_id`), so the same code signs correctly on every network; pass
`expected_chain_id` to `connect` to refuse the wrong one.

Signers that hold their keys elsewhere (a vault, a hardware key) sign
`PreparedTransaction::digest` and hand the signatures back through `with_signatures`.

Methods of a chain's own `database` API that have no typed request here are reachable through
`DatabaseApi::call`, with the generated `rpc::database::<method>` parameters and parser.

## Sending without holding the session

`NetworkBroadcastApi::submit_transaction_with_callback` waits only until the node accepts or
refuses a transaction. `poll_broadcast_confirmation` then waits a bounded time for its
confirmation, which carries the operation results (blocks do not). Between polls the session
serves other calls; notices that arrive meanwhile are buffered.

A transaction has an id before it is sent (`PreparedTransaction::id`, `transaction_id`), and
`PreparedTransaction::head_block_number` is the block a search for it starts after.
`DatabaseApi::wait_for_inclusion` and `find_transaction_in_block` look for it without sending it
again, e.g. after a crash, from a signed transaction kept as JSON
(`SignedTransactionEnvelope::from_signed_transaction`). `TransactionBuilder::max_fee` refuses a
transaction the node prices above a limit.
