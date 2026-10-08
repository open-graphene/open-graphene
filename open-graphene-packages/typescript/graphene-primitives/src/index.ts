declare const brand: unique symbol;
export type Brand<T, Name extends string> = T & { readonly [brand]: Name };
export type ObjectId = Brand<string, 'ObjectId'>;
export type PublicKey = Brand<string, 'PublicKey'>;
export type TimePointSec = Brand<string, 'TimePointSec'>;
export type VoteId = Brand<string, 'VoteId'>;
export interface ObjectIdParts {
  readonly space: number;
  readonly type: number;
  readonly instance: bigint;
}

/** Largest instance in Graphene's 8-bit space, 8-bit type, 48-bit instance ID. */
export const MAX_OBJECT_ID_INSTANCE = 0xffffffffffffn;

/** Parse canonical decimal components without leading zeros. */
export function parseObjectId(
  value: unknown,
  space?: number,
  type?: number,
): ObjectIdParts {
  if (typeof value !== 'string') {
    throw new Error('Invalid object ID');
  }

  const match =
    /^(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,2})\.(0|[1-9][0-9]{0,14})$/.exec(
      value,
    );
  if (!match || match[0] !== value) {
    throw new Error('Expected a canonical object ID');
  }

  const parsedSpace = Number(match[1]);
  const parsedType = Number(match[2]);
  const instance = BigInt(match[3]!);

  if (
    parsedSpace > 255 ||
    parsedType > 255 ||
    instance > MAX_OBJECT_ID_INSTANCE ||
    (space !== undefined && parsedSpace !== space) ||
    (type !== undefined && parsedType !== type)
  ) {
    throw new Error('Object ID has an invalid range or object type');
  }

  return {
    space: parsedSpace,
    type: parsedType,
    instance,
  };
}

export function objectId(value: string): ObjectId {
  parseObjectId(value);
  return value as ObjectId;
}
export function typedId<Name extends string>(
  value: string,
  space: number,
  type: number,
): Brand<string, Name> {
  parseObjectId(value, space, type);
  return value as Brand<string, Name>;
}

export function integer(value: unknown, min: number, max: number): number {
  if (
    typeof value !== 'number' ||
    !Number.isSafeInteger(value) ||
    value < min ||
    value > max
  ) {
    throw new Error(`Expected integer in [${min}, ${max}]`);
  }
  return value;
}

export function parseTimePointSec(value: unknown): number {
  if (
    typeof value !== 'string' ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$/.test(value)
  ) {
    throw new Error('Expected UTC time_point_sec without suffix or fraction');
  }
  const ms = Date.parse(value + 'Z');
  if (
    !Number.isFinite(ms) ||
    ms < 0 ||
    ms / 1000 > 0xffffffff ||
    new Date(ms).toISOString().slice(0, 19) !== value
  ) {
    throw new Error('Invalid time_point_sec date or range');
  }
  return ms / 1000;
}

export function parseVoteId(value: unknown): number {
  if (typeof value !== 'string' || !/^\d+:\d+$/.test(value))
    throw new Error('Invalid vote ID');
  const [kind, instance] = value.split(':').map(Number);
  return integer(kind, 0, 255) + integer(instance, 0, 0xffffff) * 256;
}

export function bytesToHex(value: Uint8Array): string {
  return Array.from(value, (byte) => byte.toString(16).padStart(2, '0')).join(
    '',
  );
}
export function hexToBytes(value: unknown, length?: number): Uint8Array {
  if (typeof value !== 'string' || !/^(?:[0-9a-fA-F]{2})*$/.test(value))
    throw new Error('Invalid hex bytes');
  const bytes = Uint8Array.from(value.match(/../g) ?? [], (pair) =>
    Number.parseInt(pair, 16),
  );
  if (length !== undefined && bytes.length !== length)
    throw new Error(`Expected ${length} bytes`);
  return bytes;
}

export interface HeadFreshnessLimits {
  readonly maxHeadAgeSeconds: number;
  readonly maxHeadTimeAheadSeconds: number;
}

/** Check a decoded head against explicit age and clock-lead limits. */
export function isHeadFresh(
  head: {
    readonly head_block_number: number;
    readonly time: string;
  },
  limits: HeadFreshnessLimits,
): boolean {
  if (
    !Number.isSafeInteger(limits.maxHeadAgeSeconds) ||
    limits.maxHeadAgeSeconds < 0 ||
    !Number.isSafeInteger(limits.maxHeadTimeAheadSeconds) ||
    limits.maxHeadTimeAheadSeconds < 0
  ) {
    throw new RangeError('Invalid head freshness limits');
  }

  const ageSeconds = Date.now() / 1000 - parseTimePointSec(head.time);
  return (
    Number.isSafeInteger(head.head_block_number) &&
    head.head_block_number >= 1 &&
    head.head_block_number <= 0xffffffff &&
    ageSeconds <= limits.maxHeadAgeSeconds &&
    ageSeconds >= -limits.maxHeadTimeAheadSeconds
  );
}

/** Equality of decoded head positions does not establish an atomic snapshot. */
export function isSameHead(
  before: {
    readonly head_block_number: number;
    readonly head_block_id: Uint8Array;
  },
  after: {
    readonly head_block_number: number;
    readonly head_block_id: Uint8Array;
  },
): boolean {
  return (
    before.head_block_number === after.head_block_number &&
    bytesToHex(before.head_block_id) === bytesToHex(after.head_block_id)
  );
}
