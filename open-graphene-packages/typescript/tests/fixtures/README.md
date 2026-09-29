# Protocol reference vectors

`protocol-vectors.json` is shared by Rust integration tests and TypeScript tests.
All private key material here is deliberately public test data (`01` repeated 32
times), unrelated to an account or environment secret. Tests never load `.env`.

`baseline.json` freezes the existing two specifications, operation tags, RPC
inventory, and source revisions at the start of the TypeScript implementation.
It is evidence of that baseline, not a second editable protocol specification.

Sources and resolutions:

- Generic object ID: pinned Swaplock `libraries/protocol/include/graphene/protocol/object_id.hpp`,
  also BitShares `ObjectId.js`. The Swaplock file was byte-compared with the
  checked-in source archive. The numeric packing and little-endian u64 give
  `1.2.345 -> 5901000000000201`. Typed account IDs use `d902`. This exposed and
  fixed the Rust generator's accidental instance-only encoding of generic IDs.
- Integer bytes follow fixed-width little-endian C++/Rust encoding. The decimal
  input strings deliberately retain values beyond JavaScript's safe number range.
- Transfer bytes: existing Rust `tests/fc.rs` and the historical offline
  bitsharesjs vector documented in `SIGNATURE-FORMAT.md`. The digest is SHA256 of
  the current chain ID followed by those bytes, checked independently in Rust
  and TypeScript. No new JS/C++ execution is claimed by extracting those vectors.
- Signature: existing public `[1; 32]` Rust signing fixture, with its own historical
  digest (not the digest of the current-chain transfer above). Verified by signing
  and recovering its public key without secrets. Current Swaplock FC revision
  `21b82e33b95e1183fc8b3a355974e06dada84d63`,
  `src/crypto/elliptic_common.cpp::public_key::is_canonical`, requires low-S.
  The original bitsharesjs `signature.js` additionally requires 32-byte DER r/s
  components. These are separate signature profiles, not an unresolved conflict.
- Access extension and snapshot digest: existing C++ wire fixtures in Swaplock
  binding `tests/fc.rs`. Sparse extension encoding is count, field index, payload;
  the expected-access-state hash is a protocol string, not decoded digest bytes.

Unresolved IR types and unsupported FC values remain explicit capability gaps;
they must never be converted into successful serializers merely to compile TS.
