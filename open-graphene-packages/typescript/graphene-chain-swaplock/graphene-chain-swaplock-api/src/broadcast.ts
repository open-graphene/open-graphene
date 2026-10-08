import * as b from '@open-graphene/chain-swaplock-bindings';
import { smallInteger, type WireValue } from '@open-graphene/codec';
import { parseTimePointSec } from '@open-graphene/primitives';
import {
  RpcRemoteError,
  RpcTransportError,
  RpcSubscriptionTimeoutError,
  type GrapheneSession,
  type SharedSubscription,
} from '@open-graphene/transport';
import { BroadcastOutcomeUnknown, type SignedTransfer } from './index.js';

export class TransactionBroadcastError extends Error {
  readonly name = 'TransactionBroadcastError';

  constructor(
    readonly code:
      | 'expired'
      | 'invalid-confirmation'
      | 'transaction-mismatch'
      | 'timeout'
      | 'closed',
    message: string,
  ) {
    super(message);
  }
}

/** A node callback, not proof of canonical inclusion or irreversible execution. */
export interface BroadcastConfirmation {
  readonly id: string;
  readonly blockNumber: number;
  readonly transactionIndex: number;
  readonly transaction: b.ProcessedTransaction;
}

function decodeConfirmation(
  notice: WireValue,
  submitted: SignedTransfer,
): BroadcastConfirmation {
  if (Array.isArray(notice) && notice.length !== 1) {
    throw new TransactionBroadcastError(
      'invalid-confirmation',
      'Expected one broadcast confirmation',
    );
  }

  const value = Array.isArray(notice) ? notice[0] : notice;
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TransactionBroadcastError(
      'invalid-confirmation',
      'Malformed broadcast confirmation',
    );
  }

  if (value.id !== submitted.id) {
    throw new TransactionBroadcastError(
      'transaction-mismatch',
      'Broadcast confirmation identifies another transaction',
    );
  }

  const blockNumber = smallInteger(1, 0xffffffff).decode(value.block_num!);
  const transactionIndex = smallInteger(0, 0xffffffff).decode(value.trx_num!);
  const transaction = b.ProcessedTransactionCodec.decode(value.trx);
  const confirmedSigned: b.SignedTransaction = {
    ref_block_num: transaction.ref_block_num,
    ref_block_prefix: transaction.ref_block_prefix,
    expiration: transaction.expiration,
    operations: transaction.operations,
    extensions: transaction.extensions,
    signatures: transaction.signatures,
  };
  const confirmedBytes = b.encodeSignedTransaction(confirmedSigned);
  const submittedBytes = b.encodeSignedTransaction(submitted.transaction);
  const sameTransaction =
    confirmedBytes.length === submittedBytes.length &&
    confirmedBytes.every((byte, index) => byte === submittedBytes[index]);

  if (!sameTransaction) {
    throw new TransactionBroadcastError(
      'transaction-mismatch',
      'Broadcast confirmation contains a different signed transaction',
    );
  }

  if (transaction.operation_results.length !== transaction.operations.length) {
    throw new TransactionBroadcastError(
      'invalid-confirmation',
      'Broadcast confirmation has an invalid operation result count',
    );
  }

  return {
    id: submitted.id,
    blockNumber,
    transactionIndex,
    transaction,
  };
}

export async function sendTransactionWithCallback(
  rpc: GrapheneSession,
  transaction: SignedTransfer,
) {
  const expiresAt = parseTimePointSec(transaction.transaction.expiration);
  if (expiresAt <= Date.now() / 1000) {
    throw new TransactionBroadcastError(
      'expired',
      'Signed transaction expired',
    );
  }

  let stream: SharedSubscription;
  try {
    stream = await rpc.subscribe(
      'network_broadcast',
      'broadcast_transaction_with_callback',
      (id) => [id, transaction.toJSON()],
    );
  } catch (error) {
    if (
      error instanceof RpcRemoteError ||
      (error instanceof RpcTransportError && !error.sent)
    ) {
      throw error;
    }

    throw new BroadcastOutcomeUnknown(transaction.id);
  }

  return {
    transactionId: transaction.id,
    close: () => stream.close(),
    wait: async (timeoutMs = 60000): Promise<BroadcastConfirmation> => {
      try {
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1) {
          throw new Error('Invalid broadcast confirmation timeout');
        }

        let notice: IteratorResult<WireValue>;
        try {
          notice = await stream.nextTimeout(timeoutMs);
        } catch (error) {
          if (
            error instanceof RpcSubscriptionTimeoutError ||
            (error instanceof Error &&
              error.name === 'RpcSubscriptionTimeoutError')
          ) {
            throw new TransactionBroadcastError(
              'timeout',
              'Broadcast confirmation timed out; submission outcome is unknown',
            );
          }

          throw new BroadcastOutcomeUnknown(transaction.id);
        }

        if (notice.done) {
          throw new TransactionBroadcastError(
            'closed',
            'Broadcast callback closed; submission outcome is unknown',
          );
        }

        try {
          return decodeConfirmation(notice.value, transaction);
        } catch (error) {
          if (error instanceof TransactionBroadcastError) {
            throw error;
          }

          throw new TransactionBroadcastError(
            'invalid-confirmation',
            'Malformed broadcast confirmation',
          );
        }
      } finally {
        stream.close();
      }
    },
  };
}
