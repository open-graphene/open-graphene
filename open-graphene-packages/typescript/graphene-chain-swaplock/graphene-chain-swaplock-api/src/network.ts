export function requireChainId(value: unknown): string {
  if (typeof value !== 'string' || !/^[0-9a-f]{64}$/.test(value)) {
    throw new Error('Expected a lowercase 32-byte chain ID');
  }

  return value;
}
