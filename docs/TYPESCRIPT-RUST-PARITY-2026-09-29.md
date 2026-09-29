# TypeScript / Rust capability parity — 2026-09-29

Implemented the remaining native TypeScript capability groups for Swaplock and
BitShares. Both languages consume the same extracted protocol specifications.
This report supersedes the transfer-only scope in earlier dated reports.

## Capability matrix

| Rust capability | TypeScript implementation | Validation |
|---|---|---|
| Generated protocol types, JSON, IDs, RPC | Shared IR → checked TS codecs, typed descriptors and bound API methods | Generator drift check, type tests, live Node/Chromium |
| FC and operation construction | Source-derived schema and checked encoders; generated factories for all nonvirtual operations | 88 Swaplock + 71 BitShares operations, each with baseline and rich payload |
| Transaction composition and fees | TransactionBuilder, recursive proposal fees, asset selection, expiration, aggregate cap | Proposal/fee tests; live compound transactions |
| Signing and broadcast | Immutable preparation, deduplicated multiple signers, expected-key WIF signing, chain signing profiles, callback and inclusion | Signature tests, local BitShares RPC harness, Swaplock live |
| Transfer helpers | Raw/decimal asset amounts, fee asset, memo, balance checks, weighted direct active keys | Unit tests and live encrypted memo transfer |
| Memo / wallet / login | ECDH/AES-CBC memo; key, signature, address, brain key, account-role and hash helpers | Independent Rust ciphertext/login vectors; wrong-key/tamper tests |
| Core utilities | Exact decimal formatting, names, balances, weighted authority analysis | Boundary and authority tests |
| Session and endpoint selection | Chain pinning, latency probes, fallback, reconnect, bounded read retries | Simulated failures and live reconnect |
| Subscriptions and live cache | Callback routing, shared database channel, account/balance/order/history/asset/head watches, market stream, ChainStore | Early notices, timeout/overflow/disconnect tests; live notices |
| Swaplock access guards | Canonical room-state digest, guard attachment, member references and key helpers | Rust/native digest vector; guarded member mutation live |
| Chain facade | `@open-graphene/graphene`, shared `@open-graphene/core` | Workspace build/typecheck |

Operation factories use typed protocol objects and a shared transaction builder;
they do not reproduce Rust's per-request fluent syntax. Generated bindings expose
95 Swaplock and 78 BitShares variants including virtual operations; signing
supports the 88 and 71 nonvirtual variants respectively.

## Independent serialization evidence

All **318 vectors** matched the Rust FC oracle and the native C++ RPC serializer:
176 Swaplock and 142 BitShares. Rich vectors exercise populated optionals, memo,
nonempty containers, integers above 2^53 and the supported sparse extensions.
Committed fixtures are in `typescript/tests/fixtures/*-fc-parity*.json`.
Chromium also checked all 159 rich operation vectors and the memo vector.

Protocol pins: Swaplock `7b471be9d1cc051e7a3563a639a9ee98979de9d8`;
BitShares `b92b82ba3e57381d111f28383e8cfa89a8356966`. The BitShares RPC inventory
was expanded from 11 to 32 methods and regenerated for both languages.

## Live validation

Swaplock's 46 non-broadcast generated RPC methods passed on node01 and node02,
in Node.js and Chromium. This includes all seven crypto methods after their
server-side enablement. Broadcast was exercised by the transactions below,
completing the **47-method inventory**.

| Transaction | Block | Scenario |
|---|---:|---|
| `68176b1648ff68b6e5bdc8f734d6f7df015cc8c7` | 1203067 | Encrypted memo transfer, strict room creation, limit order |
| `75a15bba2b95633e0ab3e3560b9dac9b35eb71dc` | 1203068 | Member addition guarded by room-state digest |
| `887994ca392fcf153622e75adb5edfd5db9c5023` | 1203069 | Member removal, order cancellation, room deletion |

All three matched native signed FC, passed node authority checks, were confirmed
by callback/history, and were independently found in node02 blocks. They were
irreversible at node02 LIB 1203238. The scenario passed 34 checks, including
balance/history/head/market/cache notices and cleanup. The transfer moved one
core atom; transaction fees totaled 262109 core atoms. A first setup attempt was
explicitly rejected because `write_policy=0` must be omitted rather than set;
it was corrected to strict policy 1. The rejected attempt remains in the journal.

BitShares read-only checks ran on `api.dex.trading` and `public.xbts.io`, in Node
and Chromium: **24 RPC methods plus native FC passed per run**. Seven crypto RPC
methods returned access denied and remain unverified live on BitShares. No
BitShares mainnet transaction was broadcast. This is an environment limitation,
not a successful crypto test; Swaplock crypto results do not replace it.

Public machine-readable evidence: [combined report](TYPESCRIPT-RUST-PARITY-2026-09-29.json).
No genesis keys or passwords are included. Test wallet fixtures use public seeds.

## Checks and boundaries

- 42 Node tests passed, including 318 FC vector comparisons; typecheck and
  Chromium tests passed.
- TypeScript generator drift check and frozen offline install passed.
- Rust workspace: 442 tests passed; formatting and all-target Clippy passed.
- No npm publication was performed.

This is capability coverage, **not 100% branch/input coverage**. It does not claim
that every operation was successfully broadcast or that all chain-state-dependent
preconditions were exhaustively tested. Rust's FC limits remain explicit:
nonempty legacy address maps and unsupported populated extension structs fail
closed. Virtual operations cannot be signed. FC agreement alone does not prove
an operation is valid for a particular on-chain state.

Streams are bounded and fail on overflow. Reconnect terminates old streams;
callers must subscribe again and obtain fresh snapshots. History watches refresh
the requested page, while Rust uses an incremental history merge internally.
Offset pages are not immutable cursors. Signers must supply the keys required by
the authority; the generic transaction path does not recursively discover keys.
JavaScript disposal clears owned byte buffers, but cannot erase immutable strings
or copies retained by callers. Acta/R-Squared remain Rust placeholders and are
outside the implemented chain scope.
