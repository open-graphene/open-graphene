// The chain the shared API in ./shared is built for. Hand-written: ./shared is generated from
// graphene-chain-api by scripts/generate.mjs and must not be edited.
import * as b from '@open-graphene/chain-mirrornet-bindings';
import type { SigningProfile } from '@open-graphene/fc/signing';

export * from '@open-graphene/chain-mirrornet-bindings';

/**
 * The signature rule of Mirrornet nodes (`fc::ecc::public_key::is_canonical` in mirrornet-fc):
 * low-S, so that deterministic signers that cannot retry, such as XKeys, are accepted.
 */
export const SIGNING_PROFILE: SigningProfile = 'low-s';

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
