declare const brand: unique symbol;
export type Brand<T, Name extends string> = T & { readonly [brand]: Name };
export type ObjectId = Brand<string, "ObjectId">;
export type PublicKey = Brand<string, "PublicKey">;
export type TimePointSec = Brand<string, "TimePointSec">;
export type VoteId = Brand<string, "VoteId">;
export interface ObjectIdParts { readonly space: number; readonly type: number; readonly instance: bigint }

export function parseObjectId(value: unknown, space?: number, type?: number): ObjectIdParts {
  if (typeof value !== "string" || !/^\d+\.\d+\.\d+$/.test(value)) throw new Error("Invalid object ID");
  const parts = value.split(".");
  const s = BigInt(parts[0]!); const t = BigInt(parts[1]!); const instance = BigInt(parts[2]!);
  if (s > 255n || t > 255n || instance > 0xffffffffffffn ||
      (space !== undefined && s !== BigInt(space)) || (type !== undefined && t !== BigInt(type))) {
    throw new Error("Object ID has an invalid range or object type");
  }
  return { space: Number(s), type: Number(t), instance };
}

export function objectId(value: string): ObjectId { parseObjectId(value); return value as ObjectId; }
export function typedId<Name extends string>(value: string, space: number, type: number): Brand<string, Name> {
  parseObjectId(value, space, type); return value as Brand<string, Name>;
}

export function integer(value: unknown, min: number, max: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < min || value > max) {
    throw new Error(`Expected integer in [${min}, ${max}]`);
  }
  return value;
}

export function parseTimePointSec(value: unknown): number {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$/.test(value)) {
    throw new Error("Expected UTC time_point_sec without suffix or fraction");
  }
  const ms = Date.parse(value + "Z");
  if (!Number.isFinite(ms) || ms < 0 || ms / 1000 > 0xffffffff || new Date(ms).toISOString().slice(0, 19) !== value) {
    throw new Error("Invalid time_point_sec date or range");
  }
  return ms / 1000;
}

export function parseVoteId(value: unknown): number {
  if (typeof value !== "string" || !/^\d+:\d+$/.test(value)) throw new Error("Invalid vote ID");
  const [kind, instance] = value.split(":").map(Number);
  return integer(kind, 0, 255) + integer(instance, 0, 0xffffff) * 256;
}

export function bytesToHex(value: Uint8Array): string {
  return Array.from(value, byte => byte.toString(16).padStart(2, "0")).join("");
}
export function hexToBytes(value: unknown, length?: number): Uint8Array {
  if (typeof value !== "string" || !/^(?:[0-9a-fA-F]{2})*$/.test(value)) throw new Error("Invalid hex bytes");
  const bytes = Uint8Array.from(value.match(/../g) ?? [], pair => Number.parseInt(pair, 16));
  if (length !== undefined && bytes.length !== length) throw new Error(`Expected ${length} bytes`);
  return bytes;
}
