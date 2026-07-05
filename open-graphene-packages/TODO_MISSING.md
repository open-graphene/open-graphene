# MISSING.md — JS SDK `bitsharesjs` vs `open-graphene-rs` 

Reference JS SDK: **`bitsharesjs`** (canonical Graphene JS lib) + **`bitsharesjs-ws`** (RPC/connection layer).
Compared against: **`open-graphene-packages/rust/graphene`** and its dependency crates
(`graphene-chain-swaplock-api`, `graphene-chain-swaplock-bindings`, `graphene-transport`, `graphene-fc`,
`graphene-core`, `graphene-primitives`).

Legend: ✅ present · ⚠️ partial · ❌ missing · 🥸 @mi4uu  

> **Headline:** the binding layer can *serialize* all 81 operation types, and the builder/connection
> ergonomics are excellent. The high-level SDK now wires up `transfer`, the **`crypto`** and grouped
> **`orders`** APIs end-to-end, **latency-sorted connection** (failover + `ConnectionStrategy`), and
> the **`PrivateKey`/`PublicKey`** foundation (WIF, derive, sign, verify, recover, shared secret),
> **`Aes` memo encrypt/decrypt**, **brain keys**, **account login** (password to keys),
> **account-name validation**, and **auto-reconnect/backoff** for read calls. Still
> missing: ergonomic per-op request builders for more of the 81 ops, the rest
> of the **ECC layer** (`Address`, random brain-key generation). The reactive object cache
> (`ChainStore`) is now in place. The engine (connection, RPC, serializer, signing, reconnect,
> validation, cache) is essentially complete; the main remaining work is ergonomic builders for
> more of the 81 operations.


---

## 1. Connection / RPC layer (`bitsharesjs-ws`)

| Feature | JS | open-graphene-rs | Notes |
|---|:--:|:--:|---|
| Connect to a node (WebSocket) | ✅ | ✅ | `Session::connect(url)` |
| `database` API (`db_api`) | ✅ | ✅ | `swaplock-api/database/` (param-shaping) + `session.database_call` (transport routing) |
| `history` API (`history_api`) | ✅ | ✅ | `swaplock-api/history/` + `session.history_call` |
| `network_broadcast` API (`network_api`) | ✅ | ✅ | `swaplock-api/network_broadcast/` + `session.network_broadcast_call` |
| `crypto` API (`crypto_api`) | ✅ | ✅ 🥸 | **DONE** (branch `feature/sdk-apis`): `CryptoApi` (7 methods: blind, blind_sum, verify_sum, verify_range, range_proof_sign, verify_range_proof_rewind, range_get_info) with hex-serde newtypes + typed results + `crypto_pedersen_commitment` example. Param-shaping in `swaplock-api/crypto/`, routed via `session.crypto_call` (transport stays ws/http only) |
| `orders` / market API (`orders_api`) | ✅ | ✅ 🥸 | **DONE** (branch `feature/sdk-apis`): grouped order book — `OrdersApi` (`tracked_groups`, `grouped_limit_orders` with optional `start`/`limit`) + `LimitOrderGroup` type + `orders_grouped_limit_orders` example. Routed via `session.orders_call`* |
| Subscriptions / `set_subscribe_callback` | ✅ | ✅ | `live.rs` + `subscribe_*` methods |
| Broadcast-with-callback (confirmation) | ✅ | ✅ | `broadcast_*_with_callback[_timeout]` |
| API id discovery / login | ✅ | ✅ | `discover_required_api` / `api_ids` |
| `ChainConfig` (chain id, address prefix) | ✅ | ⚠️ 🥸 | chain_id + prefix in builder config; no global mutable config object (by design — JS singleton is anti-idiomatic; `prefix` stays unused until `Address`/`PublicKey` serialization lands). The node-side constants are now readable via `DatabaseApi::config().get()` / `get_config()` (`get_config` RPC), **live-tested** (branch `feature/get-config`) |
| `ConnectionManager` — multi-node failover, latency sort | ✅ | ⚠️ 🥸 | **PARTIAL** (branch `feature/sdk-apis`): failover + latency sort done — `ConnectionStrategy::{FirstAvailable,LowestLatency}`, `SwaplockApi::connect_with_strategy`, `probe_latencies` (sorted fastest-first), builder `.strategy()`/`.lowest_latency()`/`.probe_latencies()`, `connection_lowest_latency` example. **Missing** for full parity: auto-reconnect/backoff (see row below) and `urlChangeCallback` |
| Auto-reconnect (`ChainWebSocket`) | ✅ | ⚠️ 🥸 | **DONE for reads** (branch `feature/auto-reconnect`): `ReconnectPolicy` (max retries + exponential backoff, capped), `GrapheneSession::reconnect()` (re-dials same node, rediscovers api ids, refuses a different chain id), and read calls (database/history/crypto/orders) auto-reconnect-and-retry on a dropped connection. **Live-tested** reconnect on the node. Mutating broadcasts are deliberately **not** auto-retried (a resend could double-submit); live subscriptions are **not** auto-resumed yet |
| Connection pool / `closeCb` lifecycle | ✅ | ⚠️ | basic session lifecycle only |

\* `orders_api` now exposes the **grouped** market order book (`grouped_limit_orders`); per-account orders also remain available via `database.get_account_orders`. The raw full order book (`get_limit_orders`) has an ergonomic builder on the database API too — `DatabaseApi::limit_orders(base, quote).limit(..).get()`, **live-tested** (branch `feature/get-limit-orders`).

---

## 2. ECC / keys (`bitsharesjs/ecc`)

| Feature | JS | open-graphene-rs | Notes |
|---|:--:|:--:|---|
| `PrivateKey` from WIF | ✅ | ✅ 🥸 | **DONE** (branches `feature/keys` + `feature/aes-memo`): `fc::PrivateKey` — `from_wif`/`from_seed`/`from_bytes`/`to_wif`/`to_public_key`/`sign`/`get_shared_secret`/`as_bytes`, secret redacted in `Debug`. Remaining: child derivation |
| `PublicKey` parse/encode | ✅ | ✅ 🥸 | **DONE** (branch `feature/keys`): `fc::PublicKey` type — `from_string`/`from_bytes`/`to_prefixed_string`/`verify`/`recover`/`as_bytes`. Remaining: `Address` string, child, `add` (next slice) |
| `Signature` (sign / verify / recover) | ✅ | ✅ 🥸 | **DONE** (branch `feature/signature-type`): `fc::Signature` type wrapping the 65-byte canonical compact signature — `sign`/`verify`/`recover`/`is_canonical`/`from_bytes`/`as_bytes`/`from_hex`/`to_hex`, re-exported through the facade, unit-tested. The free fns (`sign_digest_compact[_with_wif]`, `verify_*`, `recover_*`) remain underneath |
| Canonical signature enforcement | ✅ | ✅ | `is_graphene_canonical_compact_signature` |
| `hash` (sha256/sha512/sha1/ripemd160/hmac) | ✅ | ✅ 🥸 | **DONE** (branch `feature/hash-module`): `fc::hash` (re-exported as `graphene::hash`) — `sha256`/`sha512`/`sha1`/`ripemd160`/`hmac_sha256`/`hmac_sha512`, fixed-size arrays, unit-tested against known-answer vectors |
| `Address` (key → address string) | ✅ | ✅ 🥸 | **DONE** (branch `feature/address-type`): `fc::Address` — `from_public_key` (`RIPEMD160(SHA512(pubkey))`, verbatim bitsharesjs `toAddressString`), `to_prefixed_string`/`from_string` (4-byte RIPEMD160 checksum + prefix), `from_bytes`/`as_bytes`. Re-exported through the facade, unit-tested (round-trip + checksum) |
| `Aes` — memo encrypt/decrypt | ✅ | ✅ 🥸 | **DONE** (branch `feature/aes-memo`): `fc::{encrypt_with_checksum,decrypt_with_checksum}` + `PrivateKey::get_shared_secret` (ECDH). Verbatim port of bitsharesjs `Aes` (AES-256-CBC/PKCS7, sha512 key/iv seed, sha256 checksum). Keys cross-checked against a bitsharesjs known-answer vector. Note: on swaplock the transfer memo is a *blind* memo; this `encrypted_memo` Aes feeds `StealthConfirmation` / the standard memo format |
| `BrainKey` (generate / derive) | ✅ | ✅ 🥸 | **DONE** (branches `feature/brainkey` + `feature/suggest-brain-key`): `fc::BrainKey` — `new` (normalise) + `private_key(sequence)` = `sha256(sha512(words + " " + seq))`, verbatim bitsharesjs; plus `suggest`/`suggest_words` random **generation** (draws words from a caller-supplied dictionary via the OS CSPRNG, unbiased). Words redacted in `Debug` |
| `KeyUtils` (random key, normalize/suggest brainkey) | ✅ | ✅ 🥸 | normalise (`BrainKey::new`) + `suggest_brain_key` done (`BrainKey::suggest`, branch `feature/suggest-brain-key`). Faithful to bitsharesjs where the dictionary is a **parameter** (shipped by the wallet UI), not embedded — so no 480KB word list in the library |
| `AccountLogin` (`Login` — keys from account+password+roles) | ✅ | ✅ 🥸 | **DONE** (branch `feature/account-login`): `fc::AccountKeys::derive(account, password)` → owner/active/memo + `account_role_key` for custom roles. `sha256(normalize(account + role + password))`, verbatim bitsharesjs. **Cross-checked against the bitsharesjs `Login` test vector** (byte-identical active public key) |

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
| Byte fields JSON encoding (hex) | ✅ | ✅ 🥸 | **DONE** (branch `fix/generator-bytes-hex`): generator emits hex `serialize_with`/`deserialize_with` for `FixedBytes`/`Bytes` fields and hash static-variant payloads, so broadcasts match the node's hex JSON (was number arrays). Node-verified via the HTLC op parse |
| `precision` / `convert` (asset amount math) | ✅ | ⚠️ | `amount`, `amount_decimal`, `amount_raw` on requests; no general precision util |

---

## 4. Chain helpers (`bitsharesjs/chain`)

| Feature | JS | open-graphene-rs | Notes |
|---|:--:|:--:|---|
| `TransactionBuilder` — **any** operation | ✅ | ✅ 🥸 | **DONE** (branch `feature/transaction-builder`): `OperationsApi::transaction()` → `add_operation(Operation)` → `prepare()` (DGP header + `get_required_fees`) → `sign_with_wif` → broadcast JSON. Op JSON from serde (`[op_id, body]`). **Live-tested** (transfer accepted by the node) |
| `add_operation` for all 81 op types | ✅ | ✅ 🥸 | generic `add_operation(Operation)` takes any of the 81; fee write-back is **generated** — the bindings generator emits `Operation::set_fee`/`fee`/`name`/`is_virtual` from the spec (spec-gen reads the `// VIRTUAL` markers in `operations.hpp`), so every broadcastable op is priced automatically and the 7 virtual chain-emitted ops (`fill_order`, `htlc_redeemed`, `htlc_refund`, `asset_settle_cancel`, `credit_deal_expired`, `fba_distribute`, `execute_bid`) are rejected at `prepare()`. **Every user-signable op has an ergonomic builder** |
| `set_required_fees` (auto fee lookup) | ✅ | ✅ 🥸 | `TransactionBuilder::prepare` resolves fees for every op via `get_required_fees`, including the nested `[base_fee, [sub_fees]]` form for `proposal_create`; transfer also supports explicit `fee`/`max_fee` |
| Sign + broadcast pipeline | ✅ | ✅ 🥸 | generic via `TransactionBuilder`: `prepare → sign_with_wif → broadcast` for any supported op (plus the transfer-specific helper) |
| `propose` / proposal wrapping | ✅ | ✅ 🥸 | **DONE** (branch `feature/proposal-create`): `OperationsApi::proposal_create(payer).propose(op).expiration(..).review_period(..)` wraps any operations in a proposal, pricing each wrapped op. **Live-tested** (created proposal `1.10.0` wrapping a transfer). `proposal_update`/`proposal_delete` still binding-only |
| `ChainStore` — object cache + reactive updates | ✅ | ✅ 🥸 | **DONE** (branch `feature/chain-store`): `ChainStore` seeds from `get_objects` then a background worker applies subscription pushes to a shared id→object map; `get(id)`/`snapshot()`/`len()` read the live state. Built via `live().database().chain_store(ids)`. **Live-tested** — cached `2.1.0` head advanced on its own. Scoped to explicit object ids (not the whole-chain reactive graph) |
| `FetchChain` / `FetchChainObjects` | ✅ | ⚠️ 🥸 | typed direct getters (`get_account_by_name`, etc.) plus the generic **`get_objects`** escape hatch — `DatabaseApi::objects(ids).get()` / `get_objects(ids)` returns raw JSON for any object ids, **live-tested** (branch `feature/get-objects`); plus `DatabaseApi::block(num).get()` / `get_block(num)` returns a **typed `Option<SignedBlock>`**, with `block_header(num)` / `get_block_header(num)` returning a **typed `Option<MaybeSignedBlockHeader>`**, both `None` past the head, **live-tested** (branches `feature/get-block`/`feature/get-block-header`, typed in `feature/typed-get-block`); plus `DatabaseApi::key_references(keys).get()` / `get_key_references(keys)` returns a **typed `Vec<Vec<AccountId>>`**, **live-tested** (branch `feature/get-key-references`). `get_objects` stays raw JSON by design (arbitrary object types). Typing the block responses required fixing the generator so the `Signature` newtype deserializes from the node's hex (branch `feature/typed-get-block`) |
| `ChainValidation` (`is_account_name`, `is_cheap_name`) | ✅ | ✅ 🥸 | **DONE** (branch `feature/chain-validation`): `core::validation::{is_account_name, is_account_name_allow_short, is_cheap_name}`, verbatim bitsharesjs rules (3–63 chars, dotted labels, no `--`; `y` is a vowel), unit-tested. Chain-agnostic so it lives in core; re-exported through the SDK surface |
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
| Account (create/update/upgrade/whitelist/transfer) | ✅ (5) | ✅ 🥸 | all five wired: `account_update` (partial-update, **live-tested memo_key change→revert**, branch `feature/account-update`) + `account_create` (single-key ergonomic builder), `account_upgrade`, `account_whitelist`, `account_transfer` (branch `feature/account-ops`). The latter four are irreversible/privileged so they are **node-verified at `prepare()`** (the node prices/validates each op) and not broadcast; owner/active authority changes in `account_update` are still out of scope |
| Asset (create/update/issue/reserve/settle/publish_feed/…) | ✅ (~14) | ✅ 🥸 | all user ops wired: `issue`/`reserve`/`update` (live-tested), plus `create`, `update_issuer`, `fund_fee_pool`, `claim_pool`, `claim_fees`, `settle`, `global_settle`, `update_feed_producers`, `publish_feed`, `update_bitasset` (branch `feature/asset-ops`). **Live-tested reversible** `fund_fee_pool→claim_pool` round-trip; the MPA/irreversible ones are **node-verified at `prepare()`** and not broadcast. `asset_settle_cancel` is a **virtual** op (no builder by design) |
| Markets (limit_order create/cancel/update, call_order_update) | ✅ (4) | ✅ 🥸 | all four wired: `limit_order_create`/`cancel`/`update` (**live-tested create→update→cancel**) + `call_order_update` (`OperationsApi::call_order_update(acct).delta_collateral(..).delta_debt(..)`, node-verified — swaplock has no market-pegged asset, so the node parses the op and rejects on policy; works on a chain with a bitasset). Branch `feature/call-order-update` |
| Liquidity pools (create/delete/deposit/withdraw/exchange/update) | ✅ (6) | ✅ 🥸 | all six wired (`OperationsApi::liquidity_pool_create/delete/deposit/withdraw/exchange/update`), **live-tested full flow** create→deposit→update→exchange→withdraw→delete (pool `1.19.3`, branches `feature/liquidity-pool` + `feature/liquidity-pool-ops`) |
| HTLC (create/redeem/extend/refund) | ✅ (4) | ✅ 🥸 | all user ops wired: `htlc_create` + `htlc_redeem` + `htlc_extend` (`OperationsApi::htlc_*`), branches `feature/htlc` + `feature/htlc-extend-refund`. Serialization+signing **node-verified** (node parsed the op with hex `preimage_hash`); swaplock has **HTLC disabled at the chain level** so a full round-trip can't run here, works on an HTLC-enabled chain. `htlc_refund` is a **virtual** op (the chain emits it on expiry), so it has no broadcast builder by design |
| Credit offers / deals | ✅ (7) | ✅ 🥸 | all user ops wired (`OperationsApi::credit_offer_create/update/delete/accept`, `credit_deal_repay/update`), branch `feature/credit-offer-ops`. **Live-tested full reversible round-trip** create→update→delete (offer `1.21.0`); the borrower-side ops (`accept`, `deal_repay`, `deal_update`) need a counterpart deal so they are **node-verified at `prepare()`** and not broadcast. `auto_disable_time` defaults to just under the chain's 380-day cap (derived from head-block time). `credit_deal_expired` is a **virtual** op (no builder by design) |
| SameT funds | ✅ (5) | ✅ 🥸 | all five wired (`OperationsApi::samet_fund_create/update/delete/borrow/repay`), branch `feature/samet-fund-ops`. **Live-tested full reversible round-trip** create→update→delete (fund `1.20.0`); `borrow`/`repay` only validate as a same-transaction pair (a flash loan), so they are **node-verified at `prepare()`** and not broadcast alone (compose both into one `TransactionBuilder` for a real loan) |
| Vesting / withdraw permissions / tickets | ✅ (~9) | ✅ 🥸 | all eight wired across three modules, branch `feature/vesting-tickets-ops`. Vesting: `vesting_balance_create` (instant/linear/cdd policy) + `vesting_balance_withdraw` (**live-tested** instant create→withdraw, balance `1.13.5`). Withdraw permissions: `create`/`update`/`claim`/`delete` (**live-tested** create→update→delete, permission `1.12.0`; `claim` node-verified — needs the authorised party; `period_start_time` defaults to head-block time + 5 min since the chain requires a future start). Tickets: `ticket_create`/`ticket_update` (**node-verified** — locking funds has a cooldown that does not cleanly reverse) |
| Governance (committee/witness/worker/proposal/custom) | ✅ (~11) | ✅ 🥸 | ergonomic builders for `proposal_create` (live), plus `proposal_update`/`proposal_delete`, `committee_member_create`/`update`, `witness_create`/`update`, `worker_create` (refund/vesting/burn policy) and `custom` (branch `feature/governance-ops`). **Live-tested** proposal create→delete (`1.10.1`); the privileged/stake-heavy ops are **node-verified at `prepare()`**. `committee_member_update_global_parameters` (full chain-params, council-only) and the `custom_authority_*` ops stay binding-only by design (reachable via the generic `TransactionBuilder`) |
| Blind transfers | ✅ (3) | ✅ 🥸 | all three wired as **thin builders** (`OperationsApi::transfer_to_blind/blind_transfer/transfer_from_blind`), branch `feature/blind-transfer-ops`. The caller owns the cryptography: build the commitments, range proofs and blinding factors (via `CryptoApi`) and hand the finished bytes in; the builders shape and price the op. **Node-verified at `prepare()`** — swaplock does not expose the crypto API, so a full round-trip can't run here. `single_key_authority` reused for the output/input owner; stealth memo left `None` for now |

---

## Priority gaps (to reach JS parity)

1. **Generic `TransactionBuilder`** ✅ done (branch `feature/transaction-builder`, live-tested). Fee write-back is now fully generated (`Operation::set_fee`/`name`/`is_virtual` in the bindings); remaining: ergonomic per-op request builders if wanted.
2. ~~**`Aes` memo encrypt/decrypt**~~ ✅ done (branch `feature/aes-memo`): `encrypt_with_checksum`/`decrypt_with_checksum` + `get_shared_secret`, verbatim bitsharesjs port.
3. **`PrivateKey`/`PublicKey` types** — ✅ done (branches `feature/keys`, `feature/aes-memo`): WIF/seed/derive/sign/verify/recover/public-key-string + `get_shared_secret`. Remaining: **`Address` strings, child derivation** (unblocks `AccountLogin` #5).
4. **`ConnectionManager`** — failover + latency sort ✅ (branch `feature/sdk-apis`); **auto-reconnect + backoff ✅** for read calls (branch `feature/auto-reconnect`, live-tested). Remaining: re-subscribe live subscriptions across a reconnect, and `urlChangeCallback`.
5. **`BrainKey` + `AccountLogin`** — brain-key *derivation* ✅ (branch `feature/brainkey`) and `AccountLogin` ✅ (branch `feature/account-login`, golden-vector matched). Remaining: random brain-key **generation** (dictionary picker).
6. ~~**`ChainStore` / `FetchChain`**~~ ✅ done (branch `feature/chain-store`): reactive `ChainStore` (subscription-fed object cache) + `get_objects`/`get_block` fetch helpers. Remaining nicety: whole-chain reactive graph rather than explicit-id scope.
7. ~~**`crypto_api`**~~ ✅ done · ~~**`orders_api`** (grouped order book)~~ ✅ done — both on branch `feature/sdk-apis`. ~~Raw `get_limit_orders`~~ ✅ done (branch `feature/get-limit-orders`): ergonomic builder `DatabaseApi::limit_orders(base, quote)`, live-tested.
8. ~~**`ChainValidation`**~~ ✅ done (branch `feature/chain-validation`): `is_account_name` / `is_account_name_allow_short` / `is_cheap_name` in core, verbatim bitsharesjs.
