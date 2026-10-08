import * as b from '@open-graphene/chain-swaplock-bindings';
import { parseJson, stringifyJson } from '@open-graphene/codec';
import { transactionDigest } from '@open-graphene/fc';
import {
  encodePublicKey,
  verifyDigestCompact,
  WifSigner,
  type Signer,
} from '@open-graphene/fc/signing';
import {
  bytesToHex,
  parseTimePointSec,
  isHeadFresh,
} from '@open-graphene/primitives';
import { SignedTransfer, type SwaplockClient } from './index.js';
import { requireChainId } from './network.js';

export interface TransactionOptions {
  readonly feeAsset?: string;
  readonly expirationSeconds?: number;
  readonly maxFee?: bigint;
  /** Maximum age of the reference block; defaults to 120 seconds. */
  readonly maxHeadAgeSeconds?: number;
  /** Allowed node clock lead; defaults to 120 seconds. */
  readonly maxHeadTimeAheadSeconds?: number;
}

export class TransactionPreparationError extends Error {
  readonly name = 'TransactionPreparationError';

  constructor(
    readonly code: 'invalid-fee' | 'fee-limit' | 'invalid-head',
    message: string,
  ) {
    super(message);
  }
}

export interface TransactionSigningOptions {
  readonly signal?: AbortSignal;
}

export class TransactionSigningError extends Error {
  readonly name = 'TransactionSigningError';

  constructor(
    readonly code:
      'expired' | 'invalid-signature' | 'no-signers' | 'key-mismatch',
    message: string,
  ) {
    super(message);
  }
}

/** Stop waiting on cancellation even when an external signer never settles. */
function requestSignature(options: {
  signer: Signer;
  digest: Uint8Array;
  signal: AbortSignal | undefined;
}): Promise<Uint8Array> {
  const signal = options.signal;
  signal?.throwIfAborted();

  return new Promise((resolve, reject) => {
    const cleanup = () => signal?.removeEventListener('abort', abort);
    const abort = () => {
      cleanup();
      reject(signal?.reason);
    };
    signal?.addEventListener('abort', abort, {
      once: true,
    });

    Promise.resolve()
      .then(() => {
        signal?.throwIfAborted();
        return options.signer.signDigest(options.digest.slice());
      })
      .then(
        (signature) => {
          cleanup();
          resolve(signature);
        },
        (error) => {
          cleanup();
          reject(error);
        },
      );
  });
}

export interface PreparedTransactionOptions {
  readonly transaction: b.Transaction;
  readonly startBlock: number;
  readonly chainId: string;
}

export class PreparedTransaction {
  #wire: string;
  readonly startBlock: number;
  readonly chainId: string;

  constructor(options: PreparedTransactionOptions) {
    const transaction = options.transaction;
    this.chainId = requireChainId(options.chainId);
    this.startBlock = options.startBlock;

    b.encodeTransaction(transaction);
    this.#wire = stringifyJson(b.TransactionCodec.encode(transaction));
    Object.freeze(this);
  }
  get transaction(): b.Transaction {
    return b.TransactionCodec.decode(parseJson(this.#wire));
  }
  get bytes(): Uint8Array {
    return b.encodeTransaction(this.transaction);
  }
  async sign(
    signers: Signer | readonly Signer[],
    options: TransactionSigningOptions = {},
  ): Promise<SignedTransfer> {
    const signal = options.signal;
    signal?.throwIfAborted();
    const transaction = this.transaction;
    const expiresAt = parseTimePointSec(transaction.expiration);
    if (expiresAt <= Date.now() / 1000) {
      throw new TransactionSigningError(
        'expired',
        'Prepared transaction expired',
      );
    }

    const transactionBytes = b.encodeTransaction(transaction);
    const digest = transactionDigest(this.chainId, transactionBytes);
    const signatures: Uint8Array[] = [];
    const seen = new Set<string>();
    const signingKeys = Array.isArray(signers) ? [...signers] : [signers];

    for (const signer of signingKeys) {
      const key = Uint8Array.from(signer.publicKey);
      const id = bytesToHex(key);
      if (seen.has(id)) {
        continue;
      }

      const returnedSignature = await requestSignature({
        signer,
        digest,
        signal,
      });
      signal?.throwIfAborted();

      if (expiresAt <= Date.now() / 1000) {
        throw new TransactionSigningError(
          'expired',
          'Prepared transaction expired',
        );
      }

      const signature =
        returnedSignature instanceof Uint8Array
          ? Uint8Array.from(returnedSignature)
          : undefined;
      if (!signature || !verifyDigestCompact(digest, signature, key)) {
        throw new TransactionSigningError(
          'invalid-signature',
          'Signer returned an invalid signature',
        );
      }

      signatures.push(signature);
      seen.add(id);
    }

    if (!signatures.length) {
      throw new TransactionSigningError('no-signers', 'No signing keys given');
    }

    const signedTransaction = {
      ...transaction,
      signatures,
    };
    return new SignedTransfer(signedTransaction, this.startBlock);
  }
  async signWithWifs(
    keys: readonly { wif: string; expectedPublicKey: string }[],
    options: TransactionSigningOptions = {},
  ): Promise<SignedTransfer> {
    options.signal?.throwIfAborted();
    const signers: WifSigner[] = [];
    try {
      for (const key of keys) {
        const signer = new WifSigner(key.wif, 'swaplock-low-s');
        signers.push(signer);
        if (
          encodePublicKey(signer.publicKey, b.CHAIN.publicKeyPrefix) !==
          key.expectedPublicKey
        ) {
          throw new TransactionSigningError(
            'key-mismatch',
            'Signing key differs from expected public key',
          );
        }
      }

      return await this.sign(signers, options);
    } finally {
      for (const signer of signers) {
        signer.dispose();
      }
    }
  }
}
export class TransactionBuilder {
  #operations: b.Operation[] = [];
  #options: TransactionOptions = {};
  constructor(private readonly client: SwaplockClient) {}
  addOperation(operation: b.Operation): this {
    b.encodeOperation(operation);
    this.#operations.push(
      b.OperationCodec.decode(b.OperationCodec.encode(operation)),
    );
    return this;
  }
  feeAsset(asset: string): this {
    this.#options = { ...this.#options, feeAsset: asset };
    return this;
  }
  expiration(seconds: number): this {
    this.#options = { ...this.#options, expirationSeconds: seconds };
    return this;
  }
  maxFee(amount: bigint): this {
    this.#options = { ...this.#options, maxFee: amount };
    return this;
  }
  async prepare(): Promise<PreparedTransaction> {
    return prepareOperations(this.client, this.#operations, this.#options);
  }
}
function applyFee(operation: b.Operation, fee: b.RequiredFee): b.Operation {
  const copy = b.OperationCodec.decode(b.OperationCodec.encode(operation));
  const body = copy[1] as unknown as {
    fee: b.Asset;
    proposed_ops?: { op: b.Operation }[];
  };

  if (Array.isArray(fee)) {
    if (!body.proposed_ops || fee[1].length !== body.proposed_ops.length) {
      throw new TransactionPreparationError(
        'invalid-fee',
        'Unexpected recursive proposal fees',
      );
    }

    body.fee = fee[0];
    body.proposed_ops = body.proposed_ops.map((proposal, index) => ({
      op: applyFee(proposal.op, fee[1][index]!),
    }));
  } else {
    body.fee = fee as b.Asset;
  }

  return copy;
}

function sumFees(fee: b.RequiredFee, asset: string): bigint {
  if (Array.isArray(fee)) {
    const proposalFee = sumFees(fee[0], asset);
    const nestedFees = (fee[1] as readonly b.RequiredFee[]).reduce(
      (sum, nestedFee) => sum + sumFees(nestedFee, asset),
      0n,
    );
    return proposalFee + nestedFees;
  }

  const amount = fee as b.Asset;
  if (amount.asset_id !== asset || amount.amount < 0n) {
    throw new TransactionPreparationError(
      'invalid-fee',
      'Unexpected fee asset or negative fee',
    );
  }

  return amount.amount;
}

export async function prepareOperations(
  client: SwaplockClient,
  operations: readonly b.Operation[],
  options: TransactionOptions = {},
): Promise<PreparedTransaction> {
  if (!operations.length) {
    throw new Error('Transaction has no operations');
  }

  const expirationSeconds = options.expirationSeconds ?? 60;
  const maxFee = options.maxFee;
  const maxHeadAgeSeconds = options.maxHeadAgeSeconds ?? 120;
  const maxHeadTimeAheadSeconds = options.maxHeadTimeAheadSeconds ?? 120;
  const requestedAsset = options.feeAsset ?? '1.3.0';

  if (
    !Number.isSafeInteger(expirationSeconds) ||
    expirationSeconds < 1 ||
    expirationSeconds > 86400
  ) {
    throw new Error('Invalid transaction expiration');
  }

  if (maxFee !== undefined && (typeof maxFee !== 'bigint' || maxFee < 0n)) {
    throw new Error('Invalid maximum fee');
  }

  if (
    !Number.isSafeInteger(maxHeadAgeSeconds) ||
    maxHeadAgeSeconds < 0 ||
    !Number.isSafeInteger(maxHeadTimeAheadSeconds) ||
    maxHeadTimeAheadSeconds < 0
  ) {
    throw new Error('Invalid head freshness limits');
  }

  const original = operations.map((operation) => {
    b.encodeOperation(operation);
    return b.OperationCodec.decode(b.OperationCodec.encode(operation));
  });
  const feeAsset = requestedAsset.startsWith('1.3.')
    ? b.AssetId(requestedAsset)
    : (await client.queries.asset(requestedAsset)).id;
  const fees = await client.rpc.invoke(b.DatabaseGetRequiredFees, {
    ops: original,
    asset_symbol_or_id: feeAsset,
  });

  if (fees.length !== original.length) {
    throw new TransactionPreparationError(
      'invalid-fee',
      'Node returned wrong fee count',
    );
  }

  const totalFee = fees.reduce((sum, fee) => sum + sumFees(fee, feeAsset), 0n);
  if (maxFee !== undefined && totalFee > maxFee) {
    throw new TransactionPreparationError(
      'fee-limit',
      'Required fees exceed maximum fee',
    );
  }

  const pricedOperations = original.map((operation, index) =>
    applyFee(operation, fees[index]!),
  );
  const head = await client.rpc.invoke(
    b.DatabaseGetDynamicGlobalProperties,
    {},
  );

  const headTime = parseTimePointSec(head.time);
  const fresh = isHeadFresh(head, {
    maxHeadAgeSeconds,
    maxHeadTimeAheadSeconds,
  });
  if (!fresh) {
    throw new TransactionPreparationError(
      'invalid-head',
      'Node head is stale or ahead of the local clock',
    );
  }

  const referenceBlock = new DataView(
    head.head_block_id.buffer,
    head.head_block_id.byteOffset,
    head.head_block_id.byteLength,
  );
  const expiration = new Date((headTime + expirationSeconds) * 1000)
    .toISOString()
    .slice(0, 19) as b.TimePointSec;
  const transaction: b.Transaction = {
    ref_block_num: head.head_block_number & 0xffff,
    ref_block_prefix: referenceBlock.getUint32(4, true),
    expiration,
    operations: pricedOperations,
    extensions: [],
  };

  return new PreparedTransaction({
    transaction,
    startBlock: head.head_block_number,
    chainId: client.chainId,
  });
}
