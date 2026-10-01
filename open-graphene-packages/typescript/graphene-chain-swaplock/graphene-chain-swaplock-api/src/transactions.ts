import * as b from '@open-graphene/chain-swaplock-bindings';
import { parseJson, stringifyJson } from '@open-graphene/codec';
import { transactionDigest } from '@open-graphene/fc';
import {
  encodePublicKey,
  verifyDigestCompact,
  WifSigner,
  type Signer,
} from '@open-graphene/fc/signing';
import { bytesToHex, parseTimePointSec } from '@open-graphene/primitives';
import { SignedTransfer, type SwaplockClient } from './index.js';

export interface TransactionOptions {
  readonly feeAsset?: string;
  readonly expirationSeconds?: number;
  readonly maxFee?: bigint;
}
export class PreparedTransaction {
  #wire: string;
  constructor(
    transaction: b.Transaction,
    readonly startBlock: number,
  ) {
    b.encodeTransaction(transaction);
    this.#wire = stringifyJson(b.TransactionCodec.encode(transaction));
  }
  get transaction(): b.Transaction {
    return b.TransactionCodec.decode(parseJson(this.#wire));
  }
  get bytes(): Uint8Array {
    return b.encodeTransaction(this.transaction);
  }
  async sign(signers: Signer | readonly Signer[]): Promise<SignedTransfer> {
    const tx = this.transaction;
    if (parseTimePointSec(tx.expiration) <= Date.now() / 1000)
      throw new Error('Prepared transaction expired');
    const digest = transactionDigest(b.CHAIN.chainId, b.encodeTransaction(tx));
    const signatures: Uint8Array[] = [],
      seen = new Set<string>();
    for (const signer of Array.isArray(signers) ? signers : [signers]) {
      const key = signer.publicKey.slice(),
        id = bytesToHex(key);
      if (seen.has(id)) continue;
      const signature = await signer.signDigest(digest.slice());
      if (!verifyDigestCompact(digest, signature, key))
        throw new Error('Signer returned an invalid signature');
      signatures.push(signature);
      seen.add(id);
    }
    if (!signatures.length) throw new Error('No signing keys given');
    return new SignedTransfer({ ...tx, signatures }, this.startBlock);
  }
  async signWithWifs(
    keys: readonly { wif: string; expectedPublicKey: string }[],
  ): Promise<SignedTransfer> {
    const signers: WifSigner[] = [];
    try {
      for (const key of keys) {
        const signer = new WifSigner(key.wif, 'swaplock-low-s');
        signers.push(signer);
        if (
          encodePublicKey(signer.publicKey, b.CHAIN.publicKeyPrefix) !==
          key.expectedPublicKey
        )
          throw new Error('Signing key differs from expected public key');
      }
      return await this.sign(signers);
    } finally {
      for (const signer of signers) signer.dispose();
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
function applyFee(op: b.Operation, fee: b.RequiredFee): b.Operation {
  const copy = b.OperationCodec.decode(b.OperationCodec.encode(op));
  const body = copy[1] as unknown as {
    fee: b.Asset;
    proposed_ops?: { op: b.Operation }[];
  };
  if (Array.isArray(fee)) {
    if (!body.proposed_ops || fee[1].length !== body.proposed_ops.length)
      throw new Error('Unexpected recursive proposal fees');
    body.fee = fee[0];
    body.proposed_ops = body.proposed_ops.map((p, i) => ({
      op: applyFee(p.op, fee[1][i]!),
    }));
  } else body.fee = fee as b.Asset;
  return copy;
}
function sumFees(fee: b.RequiredFee, asset: string): bigint {
  if (Array.isArray(fee))
    return (
      sumFees(fee[0], asset) +
      (fee[1] as readonly b.RequiredFee[]).reduce(
        (sum: bigint, f: b.RequiredFee) => sum + sumFees(f, asset),
        0n,
      )
    );
  const f = fee as b.Asset;
  if (f.asset_id !== asset || f.amount < 0n)
    throw new Error('Unexpected fee asset or negative fee');
  return f.amount;
}
export async function prepareOperations(
  client: SwaplockClient,
  operations: readonly b.Operation[],
  options: TransactionOptions = {},
): Promise<PreparedTransaction> {
  if (!operations.length) throw new Error('Transaction has no operations');
  const original = operations.map((op) => {
    b.encodeOperation(op);
    return b.OperationCodec.decode(b.OperationCodec.encode(op));
  });
  const expiration = options.expirationSeconds ?? 60;
  if (!Number.isSafeInteger(expiration) || expiration < 1 || expiration > 86400)
    throw new Error('Invalid transaction expiration');
  if (
    options.maxFee !== undefined &&
    (typeof options.maxFee !== 'bigint' || options.maxFee < 0n)
  )
    throw new Error('Invalid maximum fee');
  const requestedAsset = options.feeAsset ?? '1.3.0';
  const feeAsset = requestedAsset.startsWith('1.3.')
    ? b.AssetId(requestedAsset)
    : (await client.queries.asset(requestedAsset)).id;
  const fees = await client.rpc.invoke(b.DatabaseGetRequiredFees, {
    ops: original,
    asset_symbol_or_id: feeAsset,
  });
  if (fees.length !== original.length)
    throw new Error('Node returned wrong fee count');
  const asset = (Array.isArray(fees[0]) ? fees[0][0] : fees[0]) as b.Asset;
  if (asset.asset_id !== feeAsset)
    throw new Error('Node returned wrong fee asset');
  const total = fees.reduce(
    (sum, fee) => sum + sumFees(fee, asset.asset_id),
    0n,
  );
  if (options.maxFee !== undefined && total > options.maxFee)
    throw new Error('Required fees exceed maximum fee');
  const priced = original.map((op, i) => applyFee(op, fees[i]!));
  const head = await client.rpc.invoke(
    b.DatabaseGetDynamicGlobalProperties,
    {},
  );
  if (Math.abs(Date.now() / 1000 - parseTimePointSec(head.time)) > 120)
    throw new Error('Node head time is stale');
  const tx: b.Transaction = {
    ref_block_num: head.head_block_number & 0xffff,
    ref_block_prefix: new DataView(
      head.head_block_id.buffer,
      head.head_block_id.byteOffset,
      20,
    ).getUint32(4, true),
    expiration: new Date((parseTimePointSec(head.time) + expiration) * 1000)
      .toISOString()
      .slice(0, 19) as b.TimePointSec,
    operations: priced,
    extensions: [],
  };
  return new PreparedTransaction(tx, head.head_block_number);
}
