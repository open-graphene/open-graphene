// The chain the shared API in ./shared is built for. Hand-written: ./shared is generated from
// graphene-chain-api by scripts/generate.mjs and must not be edited.
import * as b from '@open-graphene/chain-rsquared-bindings';

export * from '@open-graphene/chain-rsquared-bindings';

/** The signature rule of RSquared nodes: `fc::ecc::public_key::is_canonical`. */
export const SIGNING_PROFILE = 'graphene-legacy' as const;

/** Bytes of a block's signed header, for the inclusion fingerprint: R-Squared has no
 * `maybe_signed_block_header`, so the header is followed by the witness signature. */
export function blockHeaderBytes(block: b.SignedBlock): Uint8Array {
  const header = b.encodeBlockHeader({
    previous: block.previous,
    timestamp: block.timestamp,
    witness: block.witness,
    transaction_merkle_root: block.transaction_merkle_root,
    extensions: block.extensions,
  });
  const bytes = new Uint8Array(header.length + block.witness_signature.length);
  bytes.set(header);
  bytes.set(block.witness_signature, header.length);
  return bytes;
}
