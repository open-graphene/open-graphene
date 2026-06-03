# MISSING.md — JS SDK `bitsharesjs` vs `open-graphene-rs` 

Reference JS SDK: **`bitsharesjs`** (canonical Graphene JS lib) + **`bitsharesjs-ws`** (RPC/connection layer).
Compared against: **`open-graphene-packages/rust/graphene`** and its dependency crates
(`graphene-chain-swaplock-api`, `graphene-chain-swaplock-bindings`, `graphene-transport`, `graphene-fc`,
`graphene-core`, `graphene-primitives`).

Legend: ✅ present · ⚠️ partial · ❌ missing · 🥸 @mi4uu  

> **Headline:** the binding layer can *serialize* all 81 operation types, and the builder/connection
> ergonomics are excellent — but the **high-level SDK only wires up `transfer` end-to-end**, and the
> **ECC layer lacks memo encryption (Aes), brain keys, Address, and account login**. Everything else
> (multi-op builder, object cache, market/crypto APIs, failover) is unbuilt.


---

## 1. Connection / RPC layer (`bitsharesjs-ws`)

| Feature | JS | open-graphene-rs | Notes |
|---|:--:|:--:|---|
| Connect to a node (WebSocket) | ✅ | ✅ | `Session::connect(url)` |
| `database` API (`db_api`) | ✅ | ✅ | `graphene-transport/database_api.rs` |
| `history` API (`history_api`) | ✅ | ✅ | `graphene-transport/history_api.rs` |
| `network_broadcast` API (`network_api`) | ✅ | ✅ | `network_broadcast_api.rs` |
| `crypto` API (`crypto_api`) | ✅ | ✅ 🥸 | **DONE** (branch `feat/crypto-api`): `transport/crypto_api.rs` + `CryptoApi` (7 methods: blind, blind_sum, verify_sum, verify_range, range_proof_sign, verify_range_proof_rewind, range_get_info) + `crypto_blind_commitment` example |
| `orders` / market API (`orders_api`) | ✅ | ❌ | no `get_order_book` / `get_limit_orders` market API* |
| Subscriptions / `set_subscribe_callback` | ✅ | ✅ | `live.rs` + `subscribe_*` methods |
| Broadcast-with-callback (confirmation) | ✅ | ✅ | `broadcast_*_with_callback[_timeout]` |
| API id discovery / login | ✅ | ✅ | `discover_required_api` / `api_ids` |
| `ChainConfig` (chain id, address prefix) | ✅ | ⚠️ | chain_id + prefix in builder config; no global mutable config object |
| `ConnectionManager` — multi-node failover, latency sort | ✅ | ❌ | builder collects `servers([...])` but `connect` uses a single url |
| Auto-reconnect (`ChainWebSocket`) | ✅ | ❌ | no reconnect/retry/backoff logic |
| Connection pool / `closeCb` lifecycle | ✅ | ⚠️ | basic session lifecycle only |

\* `swaplock-api` does expose `get_account_orders` via the database API, but not a dedicated market/order-book API.

---

## 2. ECC / keys (`bitsharesjs/ecc`)

| Feature | JS | open-graphene-rs | Notes |
|---|:--:|:--:|---|
| `PrivateKey` from WIF | ✅ | ⚠️ | `fc::decode_wif_private_key` — no full struct (no `fromSeed`, `toWif`, `toPublicKey`, child derive, `get_shared_secret`) |
| `PublicKey` parse/encode | ✅ | ⚠️ | `fc::decode_public_key` / `write_public_key` — no `toAddressString`, child, `add` |
| `Signature` (sign / verify / recover) | ✅ | ⚠️ | `sign_digest_compact[_with_wif]`, `verify_*`, `recover_*` — free fns, no `Signature` type API |
| Canonical signature enforcement | ✅ | ✅ | `is_graphene_canonical_compact_signature` |
| `hash` (sha256/sha512/sha1/ripemd160/hmac) | ✅ | ⚠️ | `fc::sha` + `ripemd` dep; not full hash module |
| `Address` (key → address string) | ✅ | ❌ | only address-auth *serialization* exists, no `Address` type |
| `Aes` — memo encrypt/decrypt | ✅ | ❌ | transfer carries `encrypted_memo: Vec<u8>` but **nothing encrypts it** |
| `BrainKey` (generate / derive) | ✅ | ❌ | none |
| `KeyUtils` (random key, normalize/suggest brainkey) | ✅ | ❌ | none |
| `AccountLogin` (`Login` — keys from account+password+roles) | ✅ | ❌ | none |

---

## 3. Serializer (`bitsharesjs/serializer`)

| Feature | JS | open-graphene-rs | Notes |
|---|:--:|:--:|---|
| Operation type serialization (~80 ops) | ✅ | ✅ | **81 op structs** generated in `swaplock-bindings/generated/operations.rs` |
| Field types (`types`) | ✅ | ✅ | `generated/types.rs` |
| Static variants / extensions | ✅ | ✅ | `generated/static_variants.rs` |
| Object id encode/decode | ✅ | ✅ | `fc::{parse,write}_protocol_object_id` |
| varint / time_point / vote_id / bytes writers | ✅ | ✅ | `fc::write_*` |
| `Serializer` / `template` abstraction | ✅ | ✅ | `FcSerialize` trait |
| `SerializerValidation` | ✅ | ⚠️ | per-field validation in bindings, no standalone validation module |
| `precision` / `convert` (asset amount math) | ✅ | ⚠️ | `amount`, `amount_decimal`, `amount_raw` on requests; no general precision util |

---

## 4. Chain helpers (`bitsharesjs/chain`)

| Feature | JS | open-graphene-rs | Notes |
|---|:--:|:--:|---|
| `TransactionBuilder` — **any** operation | ✅ | ❌ | only `OperationsApi::transfer` prepare/sign/broadcast |
| `add_operation` for all 81 op types | ✅ | ❌ | bindings can serialize all 81; **none but transfer is exposed via API** |
| `set_required_fees` (auto fee lookup) | ✅ | ⚠️ | transfer supports `fee` / `max_fee`; no generic fee resolver |
| Sign + broadcast pipeline | ✅ | ✅ | transfer-only: `prepare → sign_with_wif → broadcast` |
| `propose` / proposal wrapping | ✅ | ❌ | `ProposalCreateOperation` serializable, not wired |
| `ChainStore` — object cache + reactive updates | ✅ | ❌ | none |
| `FetchChain` / `FetchChainObjects` | ✅ | ❌ | direct getters only (`get_account_by_name`, etc.) |
| `ChainValidation` (`is_account_name`, `is_cheap_name`) | ✅ | ❌ | none |
| `ObjectId` helpers | ✅ | ✅ | via `fc` |
| `NumberUtils` | ✅ | ⚠️ | amount helpers only |
| `TransactionHelper` | ✅ | ⚠️ | covered implicitly by transfer prepare |
| `EmitterInstance` (event bus) | ✅ | n/a | Rust uses typed subscriptions instead |

---

## 5. High-level operation coverage

Bindings serialize **all 81** operations. The ergonomic builder API exposes **1** (`transfer`).

| Group | Serializable (bindings) | Exposed via SDK API |
|---|:--:|:--:|
| Transfer | ✅ | ✅ |
| Account (create/update/upgrade/whitelist/transfer) | ✅ (5) | ❌ |
| Asset (create/update/issue/reserve/settle/publish_feed/…) | ✅ (~18) | ❌ |
| Markets (limit_order create/cancel/update, call_order_update) | ✅ (4) | ❌ |
| Liquidity pools (create/delete/deposit/withdraw/exchange/update) | ✅ (6) | ❌ |
| HTLC (create/redeem/extend/refund) | ✅ (4) | ❌ |
| Credit offers / deals | ✅ (7) | ❌ |
| SameT funds | ✅ (5) | ❌ |
| Vesting / withdraw permissions / tickets | ✅ (~9) | ❌ |
| Governance (committee/witness/worker/proposal/custom) | ✅ (~11) | ❌ |
| Blind transfers | ✅ (3) | ❌ |

---

## Priority gaps (to reach JS parity)

1. **Generic `TransactionBuilder`** — expose the 81 already-serializable ops through one builder (biggest leverage; the hard part is done).
2. **`Aes` memo encrypt/decrypt** — transfers already carry `encrypted_memo`; without Aes that field is unusable.
3. **Full `PrivateKey`/`PublicKey`/`Address` types** — `fromSeed`, `toWif`, `toPublicKey`, address strings, shared secret.
4. **`ConnectionManager` failover + auto-reconnect** — config already takes multiple servers; transport only uses one.
5. **`BrainKey` + `AccountLogin`** — wallet/account onboarding flows.
6. **`ChainStore` / `FetchChain`** — object cache + reactive fetch.
7. ~~**`crypto_api`**~~ ✅ done (branch `feat/crypto-api`) · **market/`orders_api`** — order book still missing.
8. **`ChainValidation`** — account-name validation helpers.
