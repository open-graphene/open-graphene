# Protocol reference vectors

`protocol-vectors.json` is shared by Rust integration tests and TypeScript tests.
All private key material here is deliberately public test data (`01` repeated 32
times), unrelated to an account or environment secret. Tests never load `.env`.

Sources and resolutions:

- Generic object ID: Graphene `libraries/protocol/include/graphene/protocol/object_id.hpp`
  (actanet-core, BitShares), also BitShares `ObjectId.js`. The numeric packing and
  little-endian u64 give `1.2.345 -> 5901000000000201`. Typed account IDs use `d902`.
  This exposed and fixed the Rust generator's accidental instance-only encoding of
  generic IDs.
- Integer bytes follow fixed-width little-endian C++/Rust encoding. The decimal
  input strings deliberately retain values beyond JavaScript's safe number range.
- Transfer bytes: existing Rust `tests/fc.rs` and the historical offline
  bitsharesjs vector documented in `SIGNATURE-FORMAT.md`. The digest is SHA256 of
  the current chain ID followed by those bytes, checked independently in Rust
  and TypeScript. No new JS/C++ execution is claimed by extracting those vectors.
- Signature: existing public `[1; 32]` Rust signing fixture, with its own historical
  digest (not the digest of the current-chain transfer above). Verified by signing
  and recovering its public key without secrets. Nodes accept only signatures
  that pass `fc::ecc::public_key::is_canonical`
  (`libraries/fc/src/crypto/elliptic_common.cpp`). BitShares and R-Squared want no
  high bit in the first byte of r and s and minimal DER, the same rule as the
  original bitsharesjs `signature.js` (`graphene-legacy`). Actanet wants low-S only
  (`low-s`). This fixture signature passes both.

- Actanet operation vectors: `node scripts/fc-parity-vectors.mjs --actanet [--rich] --live
--node=ws://127.0.0.1:8090`, compared with the Rust FC oracle and with a local
  actanet-core node (branch `launch-fixes`, with the low-S rule and the epoch and revision
  guards), as there is no public Actanet node yet. All 88 nonvirtual operations, data rooms
  and fee grants included, matched natively; the rich vectors set every guard
  (`expected_epoch`, `expected_revision`). The operations Actanet keeps unchanged from
  BitShares 7.0.4 are also checked against the natively confirmed BitShares vectors;
  `parity.test.mjs` lists the ones it adds or changes.

Unresolved IR types and unsupported FC values remain explicit capability gaps;
they must never be converted into successful serializers merely to compile TS.
