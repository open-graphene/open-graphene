# TypeScript live testing

Run commands from `open-graphene-packages/typescript`, after `pnpm build`.
These are opt-in checks. Reports distinguish failed checks, unavailable APIs and
missing fixtures; a successful process exit alone does not prove full RPC coverage.

## Full Swaplock RPC inventory

Reuse a public fixture JSON with `assetId`, `subjectRoomId` and `grantCardId`:

```sh
node scripts/test-all-live.mjs /tmp/sdk-node.json /tmp/sdk-fixtures.json
node scripts/test-all-live.mjs /tmp/sdk-browser.json /tmp/sdk-fixtures.json --browser
```

The suite reads/estimates fees through the production transport. Broadcast is
covered by a separate transaction scenario. Market and grant checks need suitable
fixtures, and results can change as market history expires.

To create fixtures, run the following only with controlled testnet accounts.
Setup broadcasts transactions and spends fees; asset creation cost 500 BTS on
the tested network. Use a new report path for each independent fixture set.

```sh
node scripts/seed-live-market.mjs /local/path/genesis.private.json /tmp/new-sdk-fixtures.json
node scripts/test-all-live.mjs /tmp/sdk-node.json /tmp/new-sdk-fixtures.json
node scripts/test-all-live.mjs /tmp/sdk-browser.json /tmp/new-sdk-fixtures.json --browser
node scripts/cleanup-live-market.mjs /local/path/genesis.private.json /tmp/new-sdk-fixtures.json
```

Setup uses native C++ serialization and local signing; it does not add TypeScript
FC coverage. Cleanup cancels the resting order. The asset, rooms, card and grant
remain labelled test fixtures. The journal prevents resubmitting unresolved sends.

## Signed transfer

```sh
# Prepare/sign and compare native serialization/authority; do not submit:
node examples/live-transfer.mjs --genesis /local/path/genesis.private.json
# Explicitly broadcast a 0.01-core-unit transfer between controlled accounts:
node examples/live-transfer.mjs --genesis /local/path/genesis.private.json --broadcast --report /tmp/transfer.json
```

The example caps fees at 3 core units, records the transaction before submission,
checks inclusion on both nodes and waits up to 60 seconds for irreversibility.
Private keys stay local and are not included in reports. An ambiguous send is
never retried automatically.

## Compound transactions, memo and room guards

```sh
node scripts/test-parity-live.mjs /local/path/genesis.private.json /tmp/parity-live.json
```

This command **broadcasts Swaplock testnet transactions**. It uses the controlled
`swaplock` and `registrar` genesis accounts and the existing market fixture asset
`1.3.100`. It tests encrypted memo, room creation, an order, guarded membership,
subscriptions and cleanup. It removes its room and cancels its order on success.
If interrupted, inspect the journal and chain state before rerunning or cleaning up.

## Independent FC comparisons

Build the Rust oracle examples, then compare generated payloads with Rust and
native node serializers. These commands do not broadcast transactions.

```sh
cargo build --offline --locked -p graphene-chain-swaplock-bindings --example fc_oracle
cargo build --offline --locked -p graphene-chain-bitshares-bindings --example fc_oracle_bitshares
node scripts/fc-parity-vectors.mjs --live
node scripts/fc-parity-vectors.mjs --rich --live
node scripts/fc-parity-vectors.mjs --bitshares --live
node scripts/fc-parity-vectors.mjs --bitshares --rich --live
```

## Recorded evidence

- [Current parity matrix, limitations and transaction IDs](TYPESCRIPT-RUST-PARITY-2026-09-29.md)
- [Initial read-only testnet validation](TYPESCRIPT-TESTNET-2026-09-29.md)
- [Initial native transfer](TYPESCRIPT-LIVE-TRANSFER-2026-09-29.md)
- [Full Swaplock RPC inventory](TYPESCRIPT-SDK-LIVE-COVERAGE-2026-09-29.md)
- [Swaplock crypto after deployment](TYPESCRIPT-SWAPLOCK-CRYPTO-LIVE-2026-09-29.md)
- [Initial BitShares API validation](TYPESCRIPT-BITSHARES-API-2026-09-29.md)
