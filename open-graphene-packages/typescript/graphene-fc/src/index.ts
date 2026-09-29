import { base58 } from "@scure/base";
import { ripemd160 } from "@noble/hashes/legacy.js";
import { sha256 } from "@noble/hashes/sha2.js";
import { hexToBytes, integer, parseObjectId, parseTimePointSec, parseVoteId } from "@open-graphene/primitives";

export { sha256 };
export function decodePublicKey(value: unknown, prefix: string): Uint8Array {
  if (typeof value !== "string" || !value.startsWith(prefix)) throw new Error("Invalid public key prefix");
  const bytes = base58.decode(value.slice(prefix.length));
  if (bytes.length !== 37) throw new Error("Invalid public key length");
  const key = bytes.slice(0, 33);
  if (!ripemd160(key).slice(0, 4).every((byte, i) => bytes[i + 33] === byte)) throw new Error("Invalid public key checksum");
  if (key[0] !== 2 && key[0] !== 3 && !key.every(byte => byte === 0)) throw new Error("Invalid compressed public key");
  return key;
}

export class FcWriter {
  private readonly output: number[] = [];
  bytes(value: Uint8Array, length?: number): void {
    if (!(value instanceof Uint8Array) || (length !== undefined && value.length !== length)) throw new Error("Invalid fixed bytes");
    for (const byte of value) this.output.push(byte);
  }
  uint(value: bigint | number, width: number): void {
    integer(width, 1, 16);
    if (typeof value === "number") integer(value, 0, Number.MAX_SAFE_INTEGER);
    let remaining = BigInt(value);
    if (remaining < 0n || remaining >= 1n << BigInt(width * 8)) throw new Error("Unsigned integer out of range");
    for (let i = 0; i < width; i++) { this.output.push(Number(remaining & 255n)); remaining >>= 8n; }
  }
  int(value: bigint | number, width: number): void {
    integer(width, 1, 16);
    if (typeof value === "number") integer(value, Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
    const n = BigInt(value); const bits = BigInt(width * 8); const limit = 1n << (bits - 1n);
    if (n < -limit || n >= limit) throw new Error("Signed integer out of range");
    this.uint(n < 0n ? (1n << bits) + n : n, width);
  }
  varint(value: bigint | number): void {
    if (typeof value === "number") integer(value, 0, Number.MAX_SAFE_INTEGER);
    let n = BigInt(value);
    if (n < 0n || n > 0xffffffffffffffffn) throw new Error("Varint out of range");
    while (n >= 128n) { this.output.push(Number(n & 127n) | 128); n >>= 7n; }
    this.output.push(Number(n));
  }
  string(value: string): void { const bytes = new TextEncoder().encode(value); this.varint(bytes.length); this.bytes(bytes); }
  objectId(value: string): void {
    const id = parseObjectId(value); this.uint((BigInt(id.space) << 56n) | (BigInt(id.type) << 48n) | id.instance, 8);
  }
  typedId(value: string, space: number, type: number): void { this.varint(parseObjectId(value, space, type).instance); }
  time(value: string): void { this.uint(parseTimePointSec(value), 4); }
  vote(value: string): void { this.uint(parseVoteId(value), 4); }
  finish(): Uint8Array { return Uint8Array.from(this.output); }
}

export function signaturePreimage(chainId: string, transaction: Uint8Array): Uint8Array {
  if (!/^[0-9a-f]{64}$/.test(chainId)) throw new Error("Chain ID must contain 64 lowercase hex characters");
  const result = new Uint8Array(32 + transaction.length); result.set(hexToBytes(chainId, 32)); result.set(transaction, 32); return result;
}
export function transactionDigest(chainId: string, transaction: Uint8Array): Uint8Array {
  return sha256(signaturePreimage(chainId, transaction));
}
