import * as b from '@open-graphene/chain-swaplock-bindings';
import { sha256 } from '@open-graphene/fc';
import { bytesToHex, isHeadFresh, isSameHead } from '@open-graphene/primitives';
import type { GrapheneSession } from '@open-graphene/transport';
import { transactionId } from './index.js';

export interface TransactionBlockInspectionOptions {
  readonly transactionId: string;
  readonly blockNumber: number;
  readonly transactionIndex: number;
  /** Maximum age of each observed head; defaults to 120 seconds. */
  readonly maxHeadAgeSeconds?: number;
  /** Allowed node clock lead; defaults to 120 seconds. */
  readonly maxHeadTimeAheadSeconds?: number;
}

/** Inclusion observed through one node, not independently verified consensus. */
export interface TransactionBlockInclusion {
  readonly transactionId: string;
  readonly blockNumber: number;
  readonly transactionIndex: number;
  /** SHA-256 of the serialized signed block header, not the protocol block ID. */
  readonly blockFingerprint: string;
  readonly observedAt: number;
  readonly irreversible: boolean;
  readonly block: b.SignedBlock;
  readonly transaction: b.ProcessedTransaction;
}

export class TransactionInspectionError extends Error {
  readonly name = 'TransactionInspectionError';

  constructor(
    readonly code: 'invalid-head' | 'head-changed',
    message: string,
  ) {
    super(message);
  }
}

function requireFreshHead(
  head: b.DynamicGlobalPropertyObject,
  limits: { maxHeadAgeSeconds: number; maxHeadTimeAheadSeconds: number },
): void {
  if (!isHeadFresh(head, limits)) {
    throw new TransactionInspectionError(
      'invalid-head',
      'The RPC head is stale or ahead of the local clock',
    );
  }
}

export async function inspectTransactionInBlock(
  rpc: GrapheneSession,
  options: TransactionBlockInspectionOptions,
): Promise<TransactionBlockInclusion | undefined> {
  const expectedTransactionId = options.transactionId;
  const blockNumber = options.blockNumber;
  const transactionIndex = options.transactionIndex;
  const maxHeadAgeSeconds = options.maxHeadAgeSeconds ?? 120;
  const maxHeadTimeAheadSeconds = options.maxHeadTimeAheadSeconds ?? 120;
  if (!/^[0-9a-f]{40}$/.test(expectedTransactionId)) {
    throw new Error('Expected a lowercase 20-byte transaction ID');
  }

  if (
    !Number.isSafeInteger(blockNumber) ||
    blockNumber < 1 ||
    blockNumber > 0xffffffff ||
    !Number.isSafeInteger(transactionIndex) ||
    transactionIndex < 0 ||
    transactionIndex > 0xffffffff
  ) {
    throw new RangeError('Invalid transaction block position');
  }

  if (
    !Number.isSafeInteger(maxHeadAgeSeconds) ||
    maxHeadAgeSeconds < 0 ||
    !Number.isSafeInteger(maxHeadTimeAheadSeconds) ||
    maxHeadTimeAheadSeconds < 0
  ) {
    throw new RangeError('Invalid head freshness limits');
  }

  const limits = {
    maxHeadAgeSeconds,
    maxHeadTimeAheadSeconds,
  };
  const before = await rpc.invoke(b.DatabaseGetDynamicGlobalProperties, {});
  requireFreshHead(before, limits);

  const block = await rpc.invoke(b.DatabaseGetBlock, {
    block_num: blockNumber,
  });
  const after = await rpc.invoke(b.DatabaseGetDynamicGlobalProperties, {});
  requireFreshHead(after, limits);

  if (!isSameHead(before, after)) {
    throw new TransactionInspectionError(
      'head-changed',
      'The chain head changed during inclusion inspection',
    );
  }

  const transaction = block?.transactions[transactionIndex];
  if (!block || !transaction || blockNumber > after.head_block_number) {
    return undefined;
  }

  const unsigned: b.Transaction = {
    ref_block_num: transaction.ref_block_num,
    ref_block_prefix: transaction.ref_block_prefix,
    expiration: transaction.expiration,
    operations: transaction.operations,
    extensions: transaction.extensions,
  };
  // A replaced position is absence of this transaction, not proof of rejection.
  if (transactionId(unsigned) !== expectedTransactionId) {
    return undefined;
  }

  const headerBytes = b.encodeMaybeSignedBlockHeader({
    previous: block.previous,
    timestamp: block.timestamp,
    witness: block.witness,
    transaction_merkle_root: block.transaction_merkle_root,
    extensions: block.extensions,
    witness_signature: block.witness_signature,
  });

  return {
    transactionId: expectedTransactionId,
    blockNumber,
    transactionIndex,
    blockFingerprint: bytesToHex(sha256(headerBytes)),
    observedAt: Date.now(),
    irreversible: after.last_irreversible_block_num >= blockNumber,
    block,
    transaction,
  };
}
