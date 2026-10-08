export {
  ChainIdMismatchError,
  hasChainIdMismatch,
} from '@open-graphene/transport';
export {
  restoreSignedTransaction,
  TransactionRestorationError,
  type RestoreSignedTransactionOptions,
} from './restoration.js';
import {
  inspectTransactionInBlock,
  type TransactionBlockInspectionOptions,
} from './inclusion.js';
export {
  TransactionInspectionError,
  type TransactionBlockInspectionOptions,
  type TransactionBlockInclusion,
} from './inclusion.js';
import { decimalToRawAmount } from '@open-graphene/core';
import { Queries } from './queries.js';
import { requireChainId } from './network.js';
export { Queries } from './queries.js';
export * from '@open-graphene/core';
import {
  TransactionBuilder,
  prepareOperations,
  type TransactionOptions,
} from './transactions.js';
export {
  TransactionPreparationError,
  TransactionSigningError,
  type TransactionSigningOptions,
  PreparedTransaction,
  type PreparedTransactionOptions,
  TransactionBuilder,
  prepareOperations,
  type TransactionOptions,
} from './transactions.js';
import * as b from '@open-graphene/chain-swaplock-bindings';
import { parseJson, stringifyJson, type WireValue } from '@open-graphene/codec';
import { bytesToHex, parseTimePointSec } from '@open-graphene/primitives';
import { sha256, transactionDigest } from '@open-graphene/fc';
import {
  encodePublicKey,
  verifyDigestCompact,
  type Signer,
} from '@open-graphene/fc/signing';
import {
  GrapheneSession,
  ChainStore,
  RpcRemoteError,
  type SessionOptions,
  type ReconnectPolicy,
} from '@open-graphene/transport';

export interface TransferRequest {
  readonly from: string;
  readonly to: string;
  /** Core asset units, not a floating point display amount. */
  readonly amount: bigint | string;
  readonly asset?: string;
  readonly feeAsset?: string;
  readonly memo?: b.MemoData;
  readonly expirationSeconds?: number;
  readonly maxFee: bigint;
}

export interface SwaplockConnectionOptions extends SessionOptions {
  readonly expectedChainId: string;
}

export interface PreparedTransferOptions {
  readonly transaction: b.Transaction;
  readonly authority: b.Authority;
  readonly startBlock: number;
  readonly chainId: string;
}
export function transactionId(transaction: b.Transaction): string {
  return bytesToHex(sha256(b.encodeTransaction(transaction)).slice(0, 20));
}
function unsigned(transaction: b.Transaction): b.Transaction {
  return {
    ref_block_num: transaction.ref_block_num,
    ref_block_prefix: transaction.ref_block_prefix,
    expiration: transaction.expiration,
    operations: transaction.operations,
    extensions: transaction.extensions,
  };
}
function header(
  head: b.DynamicGlobalPropertyObject,
  expirationSeconds = 120,
): Pick<b.Transaction, 'ref_block_num' | 'ref_block_prefix' | 'expiration'> {
  if (
    !Number.isSafeInteger(expirationSeconds) ||
    expirationSeconds < 1 ||
    expirationSeconds > 86400
  )
    throw new Error('Invalid expiration');
  const bytes = head.head_block_id;
  if (bytes.length !== 20) throw new Error('Invalid head block ID length');
  const seconds = parseTimePointSec(head.time);
  if (Math.abs(Date.now() / 1000 - seconds) > 120)
    throw new Error('Node head time is stale or local clock differs');
  return {
    ref_block_num: head.head_block_number & 0xffff,
    ref_block_prefix: new DataView(
      bytes.buffer,
      bytes.byteOffset,
      bytes.byteLength,
    ).getUint32(4, true),
    expiration: new Date((seconds + expirationSeconds) * 1000)
      .toISOString()
      .slice(0, 19) as b.TimePointSec,
  };
}
export class BroadcastOutcomeUnknown extends Error {
  constructor(readonly transactionId: string) {
    super(
      `Broadcast outcome is unknown for ${transactionId}; check inclusion before any retry`,
    );
    this.name = 'BroadcastOutcomeUnknown';
  }
}
export class SignedTransfer {
  #wire: string;
  readonly id: string;
  constructor(
    transaction: b.SignedTransaction,
    readonly startBlock: number,
  ) {
    b.encodeSignedTransaction(transaction);
    this.#wire = stringifyJson(b.SignedTransactionCodec.encode(transaction));
    this.id = transactionId(unsigned(transaction));
    Object.freeze(this);
  }
  toJSON(): WireValue {
    return parseJson(this.#wire);
  }
  get transaction(): b.SignedTransaction {
    return b.SignedTransactionCodec.decode(this.toJSON());
  }
}
export class PreparedTransfer {
  #wire: string;
  #authority: b.Authority;
  readonly id: string;
  readonly startBlock: number;
  readonly chainId: string;

  constructor(options: PreparedTransferOptions) {
    const transaction = options.transaction;
    const authority = options.authority;
    this.chainId = requireChainId(options.chainId);
    this.startBlock = options.startBlock;

    b.encodeTransaction(transaction);
    this.#wire = stringifyJson(b.TransactionCodec.encode(transaction));
    this.#authority = b.AuthorityCodec.decode(
      b.AuthorityCodec.encode(authority),
    );
    this.id = transactionId(transaction);
    Object.freeze(this);
  }
  get transaction(): b.Transaction {
    return b.TransactionCodec.decode(parseJson(this.#wire));
  }
  get bytes(): Uint8Array {
    return b.encodeTransaction(this.transaction);
  }
  async sign(input: Signer | readonly Signer[]): Promise<SignedTransfer> {
    const signers: readonly Signer[] = Array.isArray(input)
      ? input
      : [input as Signer];
    const transaction = this.transaction;
    if (parseTimePointSec(transaction.expiration) <= Date.now() / 1000) {
      throw new Error('Prepared transaction expired');
    }

    const unique = new Map<string, Signer>();
    for (const signer of signers) {
      const address = encodePublicKey(
        signer.publicKey,
        b.CHAIN.publicKeyPrefix,
      );
      unique.set(address, signer);
    }

    let weight = 0;
    for (const key of unique.keys()) {
      const contribution =
        this.#authority.key_auths.find(([k]) => k === key)?.[1] ?? 0;
      if (!contribution) {
        throw new Error('Signer is not part of active key authority');
      }

      weight += contribution;
    }

    if (weight < this.#authority.weight_threshold || !unique.size) {
      throw new Error('Signing keys do not satisfy active authority');
    }

    const transactionBytes = b.encodeTransaction(transaction);
    const digest = transactionDigest(this.chainId, transactionBytes);
    const signatures: Uint8Array[] = [];

    for (const [address, signer] of unique) {
      const key = signer.publicKey.slice();
      if (encodePublicKey(key, b.CHAIN.publicKeyPrefix) !== address) {
        throw new Error('Signer public key changed');
      }

      const signature = await signer.signDigest(digest.slice());
      if (!verifyDigestCompact(digest, signature, key)) {
        throw new Error('Signer returned an invalid signature');
      }

      signatures.push(signature);
    }

    const signedTransaction = {
      ...transaction,
      signatures,
    };
    return new SignedTransfer(signedTransaction, this.startBlock);
  }
}
export interface Inclusion {
  readonly transactionId: string;
  readonly blockNumber: number;
  readonly transactionIndex: number;
  readonly blockTime: string;
}

/** Initial vertical slice: core-asset transfer with a single active key. */
export class SwaplockClient {
  readonly queries: Queries;
  readonly database;
  readonly history;
  readonly crypto;
  readonly orders;
  readonly networkBroadcast;
  readonly operations;
  private constructor(readonly rpc: GrapheneSession) {
    this.queries = new Queries(this);
    const api = b.bindRpcApi(rpc);
    this.database = {
      ...api.database,
      account: this.queries.account.bind(this.queries),
      asset: this.queries.asset.bind(this.queries),
      accountBalances: this.queries.balances.bind(this.queries),
      accountOrders: this.queries.accountOrders.bind(this.queries),
      proposedTransactions: this.queries.proposedTransactions.bind(
        this.queries,
      ),
      watchAccount: this.queries.watchAccount.bind(this.queries),
      watchBalances: this.queries.watchBalances.bind(this.queries),
      watchAccountOrders: this.queries.watchAccountOrders.bind(this.queries),
      watchAsset: this.queries.watchAsset.bind(this.queries),
      watchDynamicGlobalProperties:
        this.queries.watchDynamicGlobalProperties.bind(this.queries),
      subscribeMarket: this.queries.subscribeMarket.bind(this.queries),
      chainStore: (ids: readonly string[]) => this.chainStore(ids),
    };
    this.history = {
      ...api.history,
      accountHistory: this.queries.accountHistory.bind(this.queries),
      watchAccountHistory: this.queries.watchAccountHistory.bind(this.queries),
    };
    this.crypto = api.crypto;
    this.orders = api.orders;
    this.networkBroadcast = {
      ...api.network_broadcast,
      sendSignedTransaction: (tx: SignedTransfer) => this.broadcast(tx),
      sendTransactionWithCallback: this.queries.broadcastWithCallback.bind(
        this.queries,
      ),
    };
    this.operations = {
      ...b.bindOperationBuilders((operation) =>
        this.transaction().addOperation(operation),
      ),
      transaction: () => this.transaction(),
    };
  }
  static async connect(
    endpoint: string | readonly string[],
    options: SwaplockConnectionOptions,
  ): Promise<SwaplockClient> {
    options?.signal?.throwIfAborted();
    const expectedChainId = requireChainId(options?.expectedChainId);
    const rpc = await GrapheneSession.connect(endpoint, {
      ...options,
      expectedChainId,
    });

    return new SwaplockClient(rpc);
  }
  async prepareTransfer(request: TransferRequest): Promise<PreparedTransfer> {
    if (
      (typeof request.amount !== 'bigint' &&
        typeof request.amount !== 'string') ||
      (typeof request.amount === 'bigint' && request.amount <= 0n)
    )
      throw new Error(
        'Transfer amount must be a positive bigint or decimal string',
      );
    if (typeof request.maxFee !== 'bigint' || request.maxFee < 0n)
      throw new Error('Maximum fee must be a nonnegative bigint');
    const accounts = await this.rpc.invoke(b.DatabaseGetAccounts, {
      account_names_or_ids: [request.from, request.to],
      subscribe: false,
    });
    const from = accounts[0];
    const to = accounts[1];
    if (!from || !to)
      throw new Error('Sender or recipient account does not exist');
    const assetInfo =
      request.asset || typeof request.amount === 'string'
        ? await this.queries.asset(request.asset ?? '1.3.0')
        : undefined;
    const asset = assetInfo?.id ?? b.AssetId('1.3.0');
    const amount =
      typeof request.amount === 'string'
        ? decimalToRawAmount(request.amount, assetInfo!.precision)
        : request.amount;
    if (amount <= 0n) throw new Error('Transfer amount must be positive');
    const feeAsset = request.feeAsset
      ? (await this.queries.asset(request.feeAsset)).id
      : b.AssetId('1.3.0');
    const operation = b.operation.transfer({
      from: from.id,
      to: to.id,
      amount: { amount, asset_id: asset },
      fee: { amount: 0n, asset_id: feeAsset },
      extensions: [],
      ...(request.memo ? { memo: request.memo } : {}),
    });
    const [fee] = await this.rpc.invoke(b.DatabaseGetRequiredFees, {
      ops: [operation],
      asset_symbol_or_id: feeAsset,
    });
    if (
      !fee ||
      Array.isArray(fee) ||
      !('amount' in fee) ||
      fee.asset_id !== feeAsset ||
      fee.amount < 0n
    )
      throw new Error('Unexpected transfer fee response');
    if (fee.amount > request.maxFee)
      throw new Error('Required fee exceeds maximum fee');
    const balances = await this.rpc.invoke(b.DatabaseGetAccountBalances, {
      account_name_or_id: from.id,
      assets: [...new Set([asset, feeAsset])],
    });
    const balance = (id: b.AssetId) =>
      balances.find((a) => a.asset_id === id)?.amount ?? 0n;
    if (
      balance(asset) < amount + (asset === feeAsset ? fee.amount : 0n) ||
      (asset !== feeAsset && balance(feeAsset) < fee.amount)
    )
      throw new Error('Insufficient liquid balance');
    const head = await this.rpc.invoke(
      b.DatabaseGetDynamicGlobalProperties,
      {},
    );
    const tx: b.Transaction = {
      ...header(head, request.expirationSeconds),
      operations: [b.operation.transfer({ ...operation[1], fee })],
      extensions: [],
    };
    return new PreparedTransfer({
      transaction: tx,
      authority: from.active,
      startBlock: head.head_block_number,
      chainId: this.chainId,
    });
  }
  async broadcast(
    transaction: SignedTransfer,
  ): Promise<{ transactionId: string; status: 'submitted' }> {
    if (
      parseTimePointSec(transaction.transaction.expiration) <=
      Date.now() / 1000
    )
      throw new Error('Signed transaction expired');
    try {
      await this.rpc.invoke(b.NetworkBroadcastBroadcastTransaction, {
        trx: transaction.toJSON(),
      });
    } catch (error) {
      if (error instanceof RpcRemoteError) throw error;
      throw new BroadcastOutcomeUnknown(transaction.id);
    }
    return { transactionId: transaction.id, status: 'submitted' };
  }
  async inspectTransactionInBlock(options: TransactionBlockInspectionOptions) {
    return inspectTransactionInBlock(this.rpc, options);
  }

  async waitForInclusion(
    transaction: SignedTransfer,
    timeoutMs = 60000,
  ): Promise<Inclusion> {
    const deadline = Date.now() + timeoutMs;
    let nextBlock = transaction.startBlock + 1;
    while (Date.now() < deadline) {
      const head = await this.rpc.invoke(
        b.DatabaseGetDynamicGlobalProperties,
        {},
      );
      for (; nextBlock <= head.head_block_number; nextBlock++) {
        const block = await this.rpc.invoke(b.DatabaseGetBlock, {
          block_num: nextBlock,
        });
        if (!block) throw new Error('Node omitted an existing block');
        for (let index = 0; index < block.transactions.length; index++) {
          const candidate = block.transactions[index]!;
          try {
            if (transactionId(unsigned(candidate)) === transaction.id)
              return {
                transactionId: transaction.id,
                blockNumber: nextBlock,
                transactionIndex: index,
                blockTime: block.timestamp,
              };
          } catch {
            /* Other transactions may use unsupported extensions. */
          }
        }
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
    throw new Error(`Inclusion not observed before timeout: ${transaction.id}`);
  }
  get chainId(): string {
    return this.rpc.chainId;
  }
  reconnect(): Promise<void> {
    return this.rpc.reconnect();
  }
  setReconnectPolicy(policy: ReconnectPolicy): void {
    this.rpc.setReconnectPolicy(policy);
  }
  static probeLatencies(
    endpoints: readonly string[],
    options: SwaplockConnectionOptions,
  ) {
    options?.signal?.throwIfAborted();
    const expectedChainId = requireChainId(options?.expectedChainId);

    return GrapheneSession.probeLatencies(endpoints, {
      ...options,
      expectedChainId,
    });
  }
  chainStore(ids: readonly string[]) {
    return ChainStore.create(this.rpc, ids);
  }
  transaction(): TransactionBuilder {
    return new TransactionBuilder(this);
  }
  prepareOperations(
    operations: readonly b.Operation[],
    options?: TransactionOptions,
  ) {
    return prepareOperations(this, operations, options);
  }
  close(): void {
    this.rpc.close();
  }
}

export * from './room-access.js';

export {
  TransactionBroadcastError,
  type BroadcastConfirmation,
} from './broadcast.js';
