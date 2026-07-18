# Graphene transaction signature format spike

This note captures the local source evidence for implementing raw Graphene/BitShares-compatible transaction signatures. It is intentionally limited to wire-level facts and API implications; it does not implement signing.

## Source evidence

### C++ Swaplock transaction digest and signing

`blockchains/swaplock/swaplock-core/libraries/protocol/transaction.cpp` shows the signature digest and signing path:

```cpp
digest_type transaction::sig_digest( const chain_id_type& chain_id )const
{
   digest_type::encoder enc;
   fc::raw::pack( enc, chain_id );
   fc::raw::pack( enc, *this );
   return enc.result();
}

const signature_type& graphene::protocol::signed_transaction::sign(
   const private_key_type& key,
   const chain_id_type& chain_id
)
{
   digest_type h = sig_digest( chain_id );
   signatures.push_back(key.sign_compact(h));
   return signatures.back();
}
```

Important implication: the mutating signed-transaction signing path signs the base `transaction` digest, not a payload that already includes signatures.

`blockchains/swaplock/swaplock-core/libraries/protocol/include/graphene/protocol/types.hpp` defines:

```cpp
using digest_type = fc::sha256;
using signature_type = fc::ecc::compact_signature;
```

`blockchains/swaplock/swaplock-core/libraries/protocol/include/graphene/protocol/transaction.hpp` reflects signed transactions as base transaction fields plus signatures:

```cpp
vector<signature_type> signatures;

FC_REFLECT_DERIVED(
  graphene::protocol::signed_transaction,
  (graphene::protocol::transaction),
  (signatures)
)
```

### BitSharesJS transaction signing path

`blockchains/bitshares/bitsharesjs/lib/chain/src/TransactionBuilder.js` signs:

```js
var sig = Signature.signBuffer(
    Buffer.concat([Buffer.from(chain_id, "hex"), this.tr_buffer]),
    private_key,
    public_key
);
this.signatures.push(sig.toBuffer());
```

`this.tr_buffer` is built from `ops.transaction.toBuffer(this)`, which is the unsigned transaction serializer.

This agrees with the C++ model and with the generated Rust helpers added in this repository:

```text
signature_preimage = chain_id_bytes || transaction_fc_bytes
signature_digest = sha256(signature_preimage)
```

### Compact signature byte layout

`blockchains/bitshares/bitsharesjs/lib/ecc/src/signature.js` defines the compact signature buffer shape:

```js
static fromBuffer(buf) {
    assert.equal(buf.length, 65, "Invalid signature length");
    i = buf.readUInt8(0);
    r = BigInteger.fromBuffer(buf.slice(1, 33));
    s = BigInteger.fromBuffer(buf.slice(33));
    return new Signature(r, s, i);
}

toBuffer() {
    var buf = Buffer.alloc(65);
    buf.writeUInt8(this.i, 0);
    this.r.toBuffer(32).copy(buf, 1);
    this.s.toBuffer(32).copy(buf, 33);
    return buf;
}
```

So `fc::ecc::compact_signature` is represented on the JS wire path as exactly:

```text
1 byte recovery/header + 32 byte r + 32 byte s
```

`blockchains/bitshares/bitsharesjs/lib/serializer/src/operations.js` confirms `signed_transaction` uses fixed-size 65-byte signatures:

```js
export const signed_transaction = new Serializer("signed_transaction", {
    ref_block_num: uint16,
    ref_block_prefix: uint32,
    expiration: time_point_sec,
    operations: array(operation),
    extensions: set(future_extensions),
    signatures: array(bytes(65))
});
```

`blockchains/bitshares/bitsharesjs/lib/serializer/src/types.js` shows fixed `bytes(65)` are written without an inner length prefix. The surrounding `array(...)` contributes only the vector length prefix.

### Canonical signature rules visible in JS

`blockchains/bitshares/bitsharesjs/lib/ecc/src/signature.js` signs a buffer by hashing it once with SHA-256, then signing the 32-byte digest:

```js
static signBuffer(buf, private_key) {
    var _hash = sha256(buf);
    return Signature.signBufferSha256(_hash, private_key);
}
```

The JS implementation searches for a canonical compact signature whose DER-encoded `r` and `s` lengths are both 32 bytes:

```js
while (true) {
    ecsignature = sign(secp256k1, buf_sha256, private_key.d, nonce++);
    der = ecsignature.toDER();
    lenR = der[3];
    lenS = der[5 + lenR];
    if (lenR === 32 && lenS === 32) {
        i = calcPubKeyRecoveryParam(...);
        i += 4;  // compressed
        i += 27; // compact
        break;
    }
}
```

`blockchains/bitshares/bitsharesjs/lib/ecc/src/ecdsa.js` also enforces low-S:

```js
if (s.compareTo(N_OVER_TWO) > 0) {
    s = n.subtract(s);
}
```

The local C++ tree calls `private_key_type::sign_compact(...)`, but the bundled `libraries/fc` implementation is not present locally, so the exact C++ nonce/canonical loop is not directly available in this checkout.

## Decisions for Rust implementation

1. Preserve the existing generated helpers:
   - `Transaction::signature_preimage_bytes()` = `chain_id_bytes || transaction_fc_bytes`
   - `Transaction::signature_digest_bytes()` = `sha256(signature_preimage)`
   - `Transaction::sign_with_wif(wif)` = compact signature bytes for the digest, returned as generated `Signature`
   - `Transaction::signed_with_wif(wif)` = one-signature generated `SignedTransaction`

2. Model Graphene signatures as raw compact bytes, not strings:
   - fixed length: 65 bytes
   - layout: `[header/recovery: u8][r: 32][s: 32]`
   - `signed_transaction.signatures` wire shape: `Vec<Signature>` where FC writes vector length, then each signature's 65 raw bytes

3. Do not serialize `TypeRef::Signature` via `String::fc_serialize`. Generated Rust now maps signatures to a dedicated `Signature` wrapper so FC serialization can enforce the fixed 65-byte compact signature shape.

4. Signing implementation uses recoverable secp256k1 ECDSA and produces Graphene-compatible compact signatures:
   - digest input is exactly `Transaction::signature_digest_bytes()`
   - libsecp256k1 normalizes signatures to low-S
   - compact header is `27 + 4 + recovery_id`
   - signatures are retried with deterministic noncedata until the compact `r/s` bytes satisfy Graphene `is_canonical` rules
   - produced signatures are returned through the generated 65-byte `Signature` wrapper
   - recovered public keys can be compared with expected compressed public keys before any broadcast path exists
   - the committed bitsharesjs fixture verifies byte-for-byte compatibility for the current signing path

5. Keep fail-closed behavior:
   - reject signature byte arrays that are not exactly 65 bytes
   - reject unknown string encodings rather than guessing
   - do not introduce `signed_transaction` FC serialization until signature bytes have a real fixed-byte representation

## BitSharesJS fixture

A local bitsharesjs fixture was generated offline from the same sample transfer transaction used by `tests/fc.rs`. The private WIF was read from the local environment and was not written to the repository. The fixture proves that generated Rust transaction bytes, preimage bytes, digest bytes, compact signature bytes, and signed-transaction bytes match the bitsharesjs serializer/signature path.

Fixture values committed in tests:

```text
transaction_hex = 0100020000000300000001000000000000000000000102a08601000000000000000000
preimage_hex = f2491c85896bb49f936152d59b850ca05bb8e09d9027eb630267697ee483b05e0100020000000300000001000000000000000000000102a08601000000000000000000
digest_hex = b459775b7ac4c1f0d1e5988d112846c0a9b75c8b8bbbd613f80499366d9be635
signature_hex = 1f4a8b0c4b54ffd78ee503c2e9112932b2caa726086aa1acb72cc550e1fea0407e4765ad8766314c4e8eb2b42017ac1048ebece448639f63d8082a1e893009a874
signed_transaction_hex = 0100020000000300000001000000000000000000000102a08601000000000000000000011f4a8b0c4b54ffd78ee503c2e9112932b2caa726086aa1acb72cc550e1fea0407e4765ad8766314c4e8eb2b42017ac1048ebece448639f63d8082a1e893009a874
```

## Implemented follow-up

The raw-signature slice is implemented in the generated Rust bindings:

- `Signature` is a generated wrapper over bytes.
- FC serialization accepts exactly 65 bytes and writes them without an inner length prefix.
- `SignedTransaction` is generated from the C++ reflected `signed_transaction` shape.
- `SignedTransaction` FC bytes are:

```text
transaction_fc_bytes || varint(signature_count) || signature_65_bytes...
```

The current live testnet transfer example is `open-graphene-packages/rust/graphene/examples/swaplock/operations_transfer_broadcast.rs`. It uses the public `graphene` facade and `graphene-chain-swaplock-api` flow to prepare a typed transfer, discover the account active public key when a single active key is present, sign locally, broadcast through `network_broadcast.broadcast_transaction`, and confirm the transfer by matching sender account history. The example loads `.env` automatically, requires `SWAPLOCK_ACTIVE_WIF` and `SWAPLOCK_ACCOUNT`, defaults `SWAPLOCK_TO_ACCOUNT` to `1.2.0`, and does not print secrets or signed transaction bytes.
