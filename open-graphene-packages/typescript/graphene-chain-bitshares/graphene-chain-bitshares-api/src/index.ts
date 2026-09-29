import * as b from '@open-graphene/chain-bitshares-bindings';
import { parseJson, stringifyJson, type WireValue } from '@open-graphene/codec';
import { bytesToHex, parseTimePointSec } from '@open-graphene/primitives';
import { sha256, transactionDigest } from '@open-graphene/fc';
import { encodePublicKey, verifyDigestCompact, isCanonicalCompactSignature, WifSigner, type Signer } from '@open-graphene/fc/signing';
import { RpcClient, RpcRemoteError, type ConnectionOptions } from '@open-graphene/transport';

/** Graphene's legacy canonical compact signature profile. */
export class BitSharesWifSigner extends WifSigner {
  constructor(wif: string) { super(wif, 'graphene-legacy'); }
}

export interface TransferRequest {
  readonly from: string;
  readonly to: string;
  /** Core asset units, not a floating point display amount. */
  readonly amount: bigint;
  readonly maxFee: bigint;
}
export function transactionId(transaction: b.Transaction): string {
  return bytesToHex(sha256(b.encodeTransaction(transaction)).slice(0, 20));
}
function unsigned(transaction: b.Transaction): b.Transaction {
  return { ref_block_num: transaction.ref_block_num, ref_block_prefix: transaction.ref_block_prefix,
    expiration: transaction.expiration, operations: transaction.operations, extensions: transaction.extensions };
}
function header(head: b.DynamicGlobalPropertyObject): Pick<b.Transaction, 'ref_block_num' | 'ref_block_prefix' | 'expiration'> {
  const bytes = head.head_block_id;
  if (bytes.length !== 20) throw new Error('Invalid head block ID length');
  const seconds = parseTimePointSec(head.time);
  if (Math.abs(Date.now() / 1000 - seconds) > 120) throw new Error('Node head time is stale or local clock differs');
  return { ref_block_num: head.head_block_number & 0xffff,
    ref_block_prefix: new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(4, true),
    expiration: new Date((seconds + 120) * 1000).toISOString().slice(0, 19) as b.TimePointSec };
}
export class BroadcastOutcomeUnknown extends Error {
  constructor(readonly transactionId: string) {
    super(`Broadcast outcome is unknown for ${transactionId}; check inclusion before any retry`); this.name = 'BroadcastOutcomeUnknown';
  }
}
export class SignedTransfer {
  #wire: string;
  readonly id: string;
  constructor(transaction: b.SignedTransaction, readonly startBlock: number) {
    if (transaction.signatures.length !== 1 || !isCanonicalCompactSignature(transaction.signatures[0]!)) {
      throw new Error('Expected one canonical BitShares signature');
    }
    b.encodeSignedTransaction(transaction);
    this.#wire = stringifyJson(b.SignedTransactionCodec.encode(transaction));
    this.id = transactionId(unsigned(transaction));
    Object.freeze(this);
  }
  toJSON(): WireValue { return parseJson(this.#wire); }
  get transaction(): b.SignedTransaction { return b.SignedTransactionCodec.decode(this.toJSON()); }
}
export class PreparedTransfer {
  #wire: string;
  #authority: b.Authority;
  readonly id: string;
  constructor(transaction: b.Transaction, authority: b.Authority, readonly startBlock: number) {
    b.encodeTransaction(transaction);
    this.#wire = stringifyJson(b.TransactionCodec.encode(transaction));
    this.#authority = b.AuthorityCodec.decode(b.AuthorityCodec.encode(authority));
    this.id = transactionId(transaction);
    Object.freeze(this);
  }
  get transaction(): b.Transaction { return b.TransactionCodec.decode(parseJson(this.#wire)); }
  get bytes(): Uint8Array { return b.encodeTransaction(this.transaction); }
  async sign(signer: Signer): Promise<SignedTransfer> {
    const tx = this.transaction;
    if (parseTimePointSec(tx.expiration) <= Date.now() / 1000) throw new Error('Prepared transaction expired');
    const publicKey = signer.publicKey.slice();
    const address = encodePublicKey(publicKey, b.CHAIN.publicKeyPrefix);
    const weight = this.#authority.key_auths.find(([key]) => key === address)?.[1] ?? 0;
    if (weight < this.#authority.weight_threshold) throw new Error('Signer does not independently satisfy active authority; multisig is not implemented');
    const digest = transactionDigest(b.CHAIN.chainId, b.encodeTransaction(tx));
    const signature = await signer.signDigest(digest.slice());
    if (!isCanonicalCompactSignature(signature)) throw new Error('BitShares requires a canonical compact signature; use BitSharesWifSigner or a compatible external signer');
    if (!verifyDigestCompact(digest, signature, publicKey)) throw new Error('Signer returned an invalid signature');
    return new SignedTransfer({ ...tx, signatures: [signature] }, this.startBlock);
  }
}
export interface Inclusion { readonly transactionId: string; readonly blockNumber: number; readonly transactionIndex: number; readonly blockTime: string }

/** Initial vertical slice: core-asset transfer with a single active key. */
export class BitSharesClient {
  private constructor(readonly rpc: RpcClient) {}
  static async connect(endpoint: string, options?: ConnectionOptions): Promise<BitSharesClient> {
    const rpc = await RpcClient.connect(endpoint, options);
    try {
      const chainId = await rpc.invoke(b.DatabaseGetChainId, {});
      if (chainId !== b.CHAIN.chainId) throw new Error('Connected node has a different chain ID');
      return new BitSharesClient(rpc);
    } catch (error) { rpc.close(); throw error; }
  }
  async prepareTransfer(request: TransferRequest): Promise<PreparedTransfer> {
    if (typeof request.amount !== 'bigint' || request.amount <= 0n) throw new Error('Transfer amount must be a positive bigint');
    if (typeof request.maxFee !== 'bigint' || request.maxFee < 0n) throw new Error('Maximum fee must be a nonnegative bigint');
    const accounts = await this.rpc.invoke(b.DatabaseGetAccounts, { account_names_or_ids: [request.from, request.to], subscribe: false });
    const from = accounts[0]; const to = accounts[1];
    if (!from || !to) throw new Error('Sender or recipient account does not exist');
    const asset = b.AssetId('1.3.0');
    const operation = b.operation.transfer({ from: from.id, to: to.id, amount: { amount: request.amount, asset_id: asset }, fee: { amount: 0n, asset_id: asset }, extensions: [] });
    const fees = await this.rpc.invoke(b.DatabaseGetRequiredFees, { ops: [operation], asset_symbol_or_id: asset });
    const fee = fees[0];
    if (!fee || Array.isArray(fee) || !('amount' in fee) || fee.asset_id !== asset || fee.amount < 0n) throw new Error('Unexpected transfer fee response');
    if (fee.amount > request.maxFee) throw new Error('Required fee exceeds maximum fee');
    const balances = await this.rpc.invoke(b.DatabaseGetAccountBalances, { account_name_or_id: from.id, assets: [asset] });
    if ((balances.find(value => value.asset_id === asset)?.amount ?? 0n) < request.amount + fee.amount) throw new Error('Insufficient liquid core balance');
    const head = await this.rpc.invoke(b.DatabaseGetDynamicGlobalProperties, {});
    const tx: b.Transaction = { ...header(head), operations: [b.operation.transfer({ ...operation[1], fee })], extensions: [] };
    return new PreparedTransfer(tx, from.active, head.head_block_number);
  }
  async broadcast(transaction: SignedTransfer): Promise<{ transactionId: string; status: 'submitted' }> {
    if (parseTimePointSec(transaction.transaction.expiration) <= Date.now() / 1000) throw new Error('Signed transaction expired');
    try { await this.rpc.invoke(b.NetworkBroadcastBroadcastTransaction, { trx: transaction.toJSON() }); }
    catch (error) { if (error instanceof RpcRemoteError) throw error; throw new BroadcastOutcomeUnknown(transaction.id); }
    return { transactionId: transaction.id, status: 'submitted' };
  }
  async waitForInclusion(transaction: SignedTransfer, timeoutMs = 60000): Promise<Inclusion> {
    const deadline = Date.now() + timeoutMs;
    let nextBlock = transaction.startBlock + 1;
    while (Date.now() < deadline) {
      const head = await this.rpc.invoke(b.DatabaseGetDynamicGlobalProperties, {});
      for (; nextBlock <= head.head_block_number; nextBlock++) {
        const block = await this.rpc.invoke(b.DatabaseGetBlock, { block_num: nextBlock });
        if (!block) throw new Error('Node omitted an existing block');
        for (let index = 0; index < block.transactions.length; index++) {
          const candidate = block.transactions[index]!;
          // Avoid attempting FC on unrelated operations not yet supported.
          if (!candidate.operations.every(op => op[0] === transaction.transaction.operations[0]![0])) continue;
          try {
            if (transactionId(unsigned(candidate)) === transaction.id) return { transactionId: transaction.id, blockNumber: nextBlock, transactionIndex: index, blockTime: block.timestamp };
          } catch { /* Other transactions may use unsupported extensions. */ }
        }
      }
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
    throw new Error(`Inclusion not observed before timeout: ${transaction.id}`);
  }
  close(): void { this.rpc.close(); }
}
