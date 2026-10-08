export class ChainIdMismatchError extends Error {
  readonly name = 'ChainIdMismatchError';
  readonly expectedChainId: string;
  readonly actualChainId: string;

  constructor(options: { expectedChainId: string; actualChainId: string }) {
    super('Connected node has a different chain ID');
    this.expectedChainId = options.expectedChainId;
    this.actualChainId = options.actualChainId;
  }
}

/** True when at least one failure reports a valid but unexpected chain ID. */
export function hasChainIdMismatch(error: unknown): boolean {
  const pending = [error];
  const visited = new Set<unknown>();

  while (pending.length > 0) {
    const current = pending.pop();
    if (current instanceof ChainIdMismatchError) {
      return true;
    }

    if (!(current instanceof AggregateError) || visited.has(current)) {
      continue;
    }

    visited.add(current);
    for (const nested of current.errors) {
      pending.push(nested);
    }
  }

  return false;
}
