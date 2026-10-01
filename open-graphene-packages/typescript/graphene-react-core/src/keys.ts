/** A tagged, deterministic representation: never put bigint or key material in query keys. */
type Encoded = readonly unknown[];
function encode(value: unknown, seen = new Set<object>()): Encoded {
  if (value === null) return ['null'];
  if (value === undefined) return ['undefined'];
  if (typeof value === 'string' || typeof value === 'boolean')
    return [typeof value, value];
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new Error('Non-finite cache value');
    return ['number', value];
  }
  if (typeof value === 'bigint') return ['bigint', value.toString()];
  if (value instanceof Uint8Array) return ['bytes', Array.from(value)];
  if (typeof value !== 'object') throw new Error('Unsupported cache value');
  if (seen.has(value)) throw new Error('Cyclic cache value');
  seen.add(value);
  try {
    if (Array.isArray(value))
      return ['array', value.map((item) => encode(item, seen))];
    if (
      Object.getPrototypeOf(value) !== Object.prototype &&
      Object.getPrototypeOf(value) !== null
    )
      throw new Error('Cache values must be plain protocol data');
    return [
      'object',
      Object.keys(value)
        .sort()
        .map((key) => [
          key,
          encode((value as Record<string, unknown>)[key], seen),
        ]),
    ];
  } finally {
    seen.delete(value);
  }
}
function decode(value: unknown): unknown {
  if (!Array.isArray(value)) throw new Error('Invalid cache encoding');
  const [tag, body] = value;
  switch (tag) {
    case 'null':
      return null;
    case 'undefined':
      return undefined;
    case 'string':
      if (typeof body === 'string') return body;
      break;
    case 'boolean':
      if (typeof body === 'boolean') return body;
      break;
    case 'number':
      if (typeof body === 'number' && Number.isFinite(body)) return body;
      break;
    case 'bigint':
      if (typeof body === 'string' && /^-?\d+$/.test(body)) return BigInt(body);
      break;
    case 'bytes':
      if (
        Array.isArray(body) &&
        body.every((n) => Number.isInteger(n) && n >= 0 && n <= 255)
      )
        return Uint8Array.from(body);
      break;
    case 'array':
      if (Array.isArray(body)) return body.map(decode);
      break;
    case 'object':
      if (Array.isArray(body)) {
        return Object.fromEntries(
          body.map((entry) => {
            if (
              !Array.isArray(entry) ||
              entry.length !== 2 ||
              typeof entry[0] !== 'string'
            )
              throw new Error('Invalid cache object');
            return [entry[0], decode(entry[1])];
          }),
        );
      }
  }
  throw new Error('Invalid cache encoding');
}
/** Serialize successful dehydrated Query data, including bigint and bytes. Not an HTML escaper. */
export function serializeCache(value: unknown): string {
  return JSON.stringify(encode(value));
}
export function deserializeCache(value: string): unknown {
  return decode(JSON.parse(value));
}
const scopes = new WeakMap<object, string>();
export function clientScope(client: object): string {
  let scope = scopes.get(client);
  if (!scope) {
    scope = crypto.randomUUID();
    scopes.set(client, scope);
  }
  return scope;
}
export interface QueryIdentity {
  readonly chainId: string;
}
/** Pass an explicit scope only when deliberately sharing data (for example SSR hydration). */
export function grapheneQueryKey(
  client: QueryIdentity,
  name: string,
  params: unknown,
  scope = clientScope(client),
) {
  return [
    'open-graphene',
    client.chainId,
    scope,
    name,
    serializeCache(params),
  ] as const;
}
