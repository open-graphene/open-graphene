// The chain the shared API in ./shared is built for. Hand-written: ./shared is generated from
// graphene-chain-api by scripts/generate.mjs and must not be edited.
import * as b from '@open-graphene/chain-bitshares-bindings';

export * from '@open-graphene/chain-bitshares-bindings';

/** The signature rule of BitShares nodes: `fc::ecc::public_key::is_canonical`. */
export const SIGNING_PROFILE = 'graphene-legacy' as const;

/** Bytes of a block's signed header, for the inclusion fingerprint. */
export function blockHeaderBytes(block: b.SignedBlock): Uint8Array {
  return b.encodeMaybeSignedBlockHeader({
    previous: block.previous,
    timestamp: block.timestamp,
    witness: block.witness,
    transaction_merkle_root: block.transaction_merkle_root,
    extensions: block.extensions,
    witness_signature: block.witness_signature,
  });
}
