import { LosslessNumber, isLosslessNumber, parse, stringify } from "lossless-json";
import { bytesToHex, hexToBytes, parseObjectId, parseTimePointSec, parseVoteId } from "@open-graphene/primitives";
import { decodePublicKey } from "@open-graphene/fc";

export type WireValue = null | boolean | string | number | LosslessNumber | readonly WireValue[] | { readonly [key: string]: WireValue };
export interface Codec<T> { decode(value: unknown): T; encode(value: T): WireValue }
export class CodecError extends Error {
  constructor(message: string, readonly path: readonly (string | number)[] = []) {
    super(`${path.length ? path.join(".") + ": " : ""}${message}`); this.name = "CodecError";
  }
}
function fail(message: string): never { throw new CodecError(message); }
function at<T>(key: string | number, action: () => T): T {
  try { return action(); } catch (error) {
    if (error instanceof CodecError) throw new CodecError(error.message.replace(/^.*?: /, ""), [key, ...error.path]);
    throw new CodecError(error instanceof Error ? error.message : "Invalid value", [key]);
  }
}
export function parseJson(text: string): WireValue { return parse(text) as WireValue; }
export function stringifyJson(value: WireValue): string {
  return stringify(value) ?? fail("Cannot serialize undefined");
}
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value) || value instanceof Uint8Array || isLosslessNumber(value)) fail("Expected object");
  return value as Record<string, unknown>;
}
function array(value: unknown): readonly unknown[] { return Array.isArray(value) ? value : fail("Expected array"); }
function numeric(value: unknown): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) fail("Unsafe or non-integer number");
    return BigInt(value);
  }
  const text = isLosslessNumber(value) ? value.value : value;
  if (typeof text !== "string" || !/^-?\d+$/.test(text)) fail("Expected integer or decimal integer string");
  return BigInt(text);
}
function codec<T>(decode: (value: unknown) => T, encode: (value: T) => WireValue): Codec<T> { return { decode, encode }; }
export function lazy<T>(factory: () => Codec<T>): Codec<T> {
  let resolved: Codec<T> | undefined;
  return { decode: value => (resolved ??= factory()).decode(value), encode: value => (resolved ??= factory()).encode(value) };
}
export const bool: Codec<boolean> = codec(value => typeof value === "boolean" ? value : fail("Expected boolean"), value => bool.decode(value));
export const text: Codec<string> = codec(value => typeof value === "string" ? value : fail("Expected string"), value => text.decode(value));
export function smallInteger(min: number, max: number): Codec<number> {
  const decode = (value: unknown): number => { const n = numeric(value); if (n < BigInt(min) || n > BigInt(max)) fail("Integer out of range"); return Number(n); };
  return codec(decode, value => decode(value));
}
export function wideInteger(bits: number, signed: boolean, decimalString = false): Codec<bigint> {
  const high = 1n << BigInt(signed ? bits - 1 : bits); const low = signed ? -high : 0n;
  const decode = (value: unknown): bigint => { const n = numeric(value); if (n < low || n >= high) fail("Integer out of range"); return n; };
  return codec(decode, value => { const n = decode(value); return decimalString ? n.toString() : new LosslessNumber(n.toString()); });
}
export function checkedString<T extends string>(validate: (value: string) => void): Codec<T> {
  const decode = (value: unknown): T => { const s = text.decode(value); validate(s); return s as T; };
  return codec(decode, value => decode(value));
}
export const objectId = (space?: number, type?: number): Codec<string> => checkedString(s => { parseObjectId(s, space, type); });
export const publicKey = (prefix: string): Codec<string> => checkedString(s => { decodePublicKey(s, prefix); });
export const timePointSec: Codec<string> = checkedString(s => { parseTimePointSec(s); });
export const voteId: Codec<string> = checkedString(s => { parseVoteId(s); });
export const fixedHex = (size: number): Codec<string> => checkedString(s => { hexToBytes(s, size); });
export function bytes(size?: number): Codec<Uint8Array> {
  const decode = (value: unknown): Uint8Array => {
    const result = typeof value === "string" ? hexToBytes(value, size)
      : value instanceof Uint8Array ? value.slice() : Uint8Array.from(array(value), n => smallInteger(0, 255).decode(n));
    if (size !== undefined && result.length !== size) fail(`Expected ${size} bytes`);
    return result;
  };
  return codec(decode, value => bytesToHex(decode(value)));
}
export function optional<T>(inner: Codec<T>): Codec<T | null> {
  return codec(value => value == null ? null : inner.decode(value), value => value == null ? null : inner.encode(value));
}
export function vector<T>(inner: Codec<T>): Codec<readonly T[]> {
  return codec(value => array(value).map((item, i) => at(i, () => inner.decode(item))),
    value => array(value).map((item, i) => at(i, () => inner.encode(item as T))));
}
export function pair<A, B>(first: Codec<A>, second: Codec<B>): Codec<readonly [A, B]> {
  const decode = (value: unknown): readonly [A, B] => { const v = array(value); if (v.length !== 2) fail("Expected pair"); return [at(0, () => first.decode(v[0])), at(1, () => second.decode(v[1]))]; };
  return codec(decode, value => {
    const v = array(value); if (v.length !== 2) fail("Expected pair");
    return [at(0, () => first.encode(v[0] as A)), at(1, () => second.encode(v[1] as B))];
  });
}
export const voidValue: Codec<Record<string, never>> = codec(value => {
  if (Object.keys(record(value)).length) fail("Expected empty void object"); return {};
}, value => { voidValue.decode(value); return {}; });
export const rpcVoid: Codec<void> = codec(value => { if (value !== null && value !== undefined) fail("Expected void RPC response"); }, () => null);
export const unknownValue: Codec<unknown> = codec(value => value, value => {
  if (value === null || typeof value === "boolean" || typeof value === "string" || isLosslessNumber(value)) return value;
  if (typeof value === "number" && Number.isFinite(value) && (!Number.isInteger(value) || Number.isSafeInteger(value))) return value;
  if (Array.isArray(value)) return value.map(item => unknownValue.encode(item));
  if (value && typeof value === "object" && Object.getPrototypeOf(value) === Object.prototype) {
    return Object.fromEntries(Object.entries(value).map(([key, v]) => [key, unknownValue.encode(v)]));
  }
  return fail("Value is not lossless JSON");
});
export function unsupported(reason: string): Codec<never> { return codec(() => fail(reason), () => fail(reason)); }
export function enumNumber(values: readonly number[], bitfield = false): Codec<number> {
  const decode = (value: unknown): number => { const n = smallInteger(-2147483648, 2147483647).decode(value); if (!bitfield && !values.includes(n)) fail("Unknown enum value"); return n; };
  return codec(decode, value => decode(value));
}

// Type erasure is confined to the runtime dispatcher. Generated declarations
// bind each checked shape back to its concrete public TypeScript type.
export interface Field { readonly name: string; readonly codec: Codec<unknown>; readonly optional?: boolean; readonly defaultValue?: unknown }
export interface StructOptions { readonly legacyEmptyArray?: boolean; readonly rejectUnknown?: boolean }
export function struct<T extends object>(fields: readonly Field[], options: StructOptions = {}): Codec<T> {
  const transform = (value: unknown, encode: boolean): Record<string, unknown> => {
    const input = options.legacyEmptyArray && Array.isArray(value) && value.length === 0 ? {} : record(value);
    if (encode || options.rejectUnknown) {
      const names = new Set(fields.map(field => field.name));
      if (Object.keys(input).some(key => !names.has(key))) fail("Unknown field in protocol object");
    }
    const entries: [string, unknown][] = [];
    for (const field of fields) {
      let member = Object.hasOwn(input, field.name) ? input[field.name] : undefined;
      if (member == null && field.optional) continue;
      if (member === undefined && Object.hasOwn(field, "defaultValue")) member = field.defaultValue;
      entries.push([field.name, at(field.name, () => encode ? field.codec.encode(member) : field.codec.decode(member))]);
    }
    return Object.fromEntries(entries);
  };
  return codec(value => transform(value, false) as T, value => {
    const result = transform(value, true) as { [key: string]: WireValue };
    return options.legacyEmptyArray && Object.keys(result).length === 0 ? [] : result;
  });
}
export function variant<T>(arms: Readonly<Record<number, Codec<unknown>>>): Codec<T> {
  const transform = (value: unknown, encode: boolean): unknown => {
    const v = array(value); if (v.length !== 2) fail("Expected [tag, payload]");
    const tag = smallInteger(0, 0xffffffff).decode(v[0]); const arm = arms[tag];
    if (!arm) fail(`Unknown static variant tag ${tag}`);
    return [tag, at(tag, () => encode ? arm.encode(v[1]) : arm.decode(v[1]))];
  };
  return codec(value => transform(value, false) as T, value => transform(value, true) as WireValue);
}
export interface RoutedObject { readonly kind: string; readonly value: unknown }
export function objectUnion<T extends RoutedObject>(routes: Readonly<Record<string, { readonly kind: string; readonly codec: Codec<unknown> }>>): Codec<T> {
  const decode = (value: unknown): T => {
    const input = record(value); const id = parseObjectId(input.id); const route = routes[`${id.space}.${id.type}`];
    return (route ? { kind: route.kind, value: route.codec.decode(input) } : { kind: "unknown", value: input }) as T;
  };
  return codec(decode, value => {
    const v = record(value); const route = Object.values(routes).find(r => r.kind === v.kind);
    if (!route) { if (v.kind !== "unknown") fail("Unknown object discriminator"); return unknownValue.encode(v.value); }
    const encoded = route.codec.encode(v.value); const decoded = decode(encoded);
    if (decoded.kind !== v.kind) fail("Object discriminator and ID mismatch"); return encoded;
  });
}
export interface RpcParam { readonly name: string; readonly codec: Codec<unknown>; readonly required: boolean; readonly nullable: boolean }
export interface RpcMethod<P, R> { readonly api: string; readonly method: string; readonly returns: Codec<R>; encodeParams(params: P): readonly WireValue[]; parseReturns(value: unknown): R }
export function rpc<P extends object, R>(api: string, method: string, params: readonly RpcParam[], returns: Codec<R>): RpcMethod<P, R> {
  return { api, method, returns, parseReturns: value => returns.decode(value), encodeParams(value) {
    const input = record(value); const names = new Set(params.map(p => p.name));
    if (Object.keys(input).some(key => !names.has(key))) fail("Unknown RPC parameter");
    let end = params.length;
    while (end > 0 && !params[end - 1]!.required && input[params[end - 1]!.name] === undefined) end--;
    return params.slice(0, end).map(p => at(p.name, () => {
      const v = input[p.name];
      if (v === undefined) return fail("Missing positional parameter");
      if (v === null && !p.nullable) return fail("RPC parameter is not nullable");
      return p.codec.encode(v);
    }));
  } };
}
export const config: Codec<Readonly<Record<string, unknown>>> = codec(value => record(value), value => unknownValue.encode(record(value)));
export function requiredFee<T>(asset: Codec<T>): Codec<T | readonly [T, readonly unknown[]]> {
  const result: Codec<T | readonly [T, readonly unknown[]]> = lazy(() => codec(value => Array.isArray(value)
    ? pair(asset, vector(result)).decode(value) : asset.decode(value), value => Array.isArray(value)
    ? pair(asset, vector(result)).encode(value as readonly [T, readonly (T | readonly [T, readonly unknown[]])[]]) : asset.encode(value as T)));
  return result;
}
