import * as b from '@open-graphene/chain-swaplock-bindings';
import { parseJson } from '@open-graphene/codec';
import { transactionDigest } from '@open-graphene/fc';
import { verifyDigestCompact } from '@open-graphene/fc/signing';
import { SignedTransfer } from './index.js';
import { requireChainId } from './network.js';

export interface RestoreSignedTransactionOptions {
  readonly serializedTransaction: string;
  readonly chainId: string;
  readonly expectedTransactionId: string;
  /** Compressed public key of the sole expected signer. */
  readonly expectedPublicKey: Uint8Array;
  /** Caller-owned search hint, not part of the signed transaction. */
  readonly startBlock: number;
}

export class TransactionRestorationError extends Error {
  readonly name = 'TransactionRestorationError';

  constructor(
    readonly code:
      | 'invalid-input'
      | 'invalid-transaction'
      | 'transaction-mismatch'
      | 'invalid-signature',
    message: string,
  ) {
    super(message);
  }
}

/** Restore one signature without network access or an expiration check. */
export function restoreSignedTransaction(
  options: RestoreSignedTransactionOptions,
): SignedTransfer {
  let chainId: string;
  try {
    chainId = requireChainId(options.chainId);
  } catch {
    throw new TransactionRestorationError('invalid-input', 'Invalid chain ID');
  }

  if (
    typeof options.expectedTransactionId !== 'string' ||
    !/^[0-9a-f]{40}$/.test(options.expectedTransactionId) ||
    !(options.expectedPublicKey instanceof Uint8Array) ||
    options.expectedPublicKey.length !== 33 ||
    !Number.isSafeInteger(options.startBlock) ||
    options.startBlock < 0 ||
    options.startBlock > 0xffffffff
  ) {
    throw new TransactionRestorationError(
      'invalid-input',
      'Invalid expected transaction ID, public key or start block',
    );
  }

  let signed: SignedTransfer;
  try {
    const transaction = b.SignedTransactionCodec.decode(
      parseJson(options.serializedTransaction),
    );
    signed = new SignedTransfer(transaction, options.startBlock);
  } catch {
    // Codec errors can contain serialized transaction data; keep it private.
    throw new TransactionRestorationError(
      'invalid-transaction',
      'Invalid serialized signed transaction',
    );
  }

  if (signed.id !== options.expectedTransactionId) {
    throw new TransactionRestorationError(
      'transaction-mismatch',
      'The saved transaction has a different ID',
    );
  }

  const transaction = signed.transaction;
  const unsigned: b.Transaction = {
    ref_block_num: transaction.ref_block_num,
    ref_block_prefix: transaction.ref_block_prefix,
    expiration: transaction.expiration,
    operations: transaction.operations,
    extensions: transaction.extensions,
  };
  const bytes = b.encodeTransaction(unsigned);
  const digest = transactionDigest(chainId, bytes);
  const signature = transaction.signatures[0];

  if (
    transaction.signatures.length !== 1 ||
    !signature ||
    !verifyDigestCompact(digest, signature, options.expectedPublicKey)
  ) {
    throw new TransactionRestorationError(
      'invalid-signature',
      'Expected exactly one valid signature for the supplied chain and public key',
    );
  }

  return signed;
}
