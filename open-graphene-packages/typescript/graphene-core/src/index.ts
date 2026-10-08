import { integer } from '@open-graphene/primitives';
export function decimalToRawAmount(value: string, precision: number): bigint {
  integer(precision, 0, 255);
  value = value.trim();
  if (!value || !/^\d*(?:\.\d*)?$/.test(value))
    throw new Error('Expected nonnegative decimal amount');
  const [whole = '', fraction = ''] = value.split('.');
  const significant = fraction.replace(/0+$/, '');
  if (significant.length > precision)
    throw new Error('Too many decimal places');
  const amount = BigInt((whole || '0') + significant.padEnd(precision, '0'));
  if (amount > 0x7fffffffffffffffn) throw new Error('Amount exceeds int64');
  return amount;
}
export function formatRawAmount(value: bigint, precision: number): string {
  integer(precision, 0, 255);
  if (typeof value !== 'bigint' || value < -(1n << 63n) || value >= 1n << 63n)
    throw new Error('Amount exceeds int64');
  const sign = value < 0n ? '-' : '',
    digits = (value < 0n ? -value : value)
      .toString()
      .padStart(precision + 1, '0');
  return (
    sign +
    (precision
      ? digits.slice(0, -precision) + '.' + digits.slice(-precision)
      : digits)
  );
}
export function isAccountName(name: string, allowShort = false): boolean {
  return (
    name.length >= (allowShort ? 1 : 3) &&
    name.length <= 63 &&
    name
      .split('.')
      .every(
        (label) =>
          /^[a-z](?:[a-z0-9-]*[a-z0-9])?$/.test(label) && !label.includes('--'),
      )
  );
}
export function isCheapName(name: string): boolean {
  return /[0-9-]/.test(name) || !/[aeiouy]/.test(name);
}
/**
 * Whether one directly listed key alone reaches the authority threshold.
 * Does not resolve account/address authorities or verify possession of a key.
 */
export function canKeySatisfyAuthority(
  authority: {
    readonly weight_threshold: number;
    readonly key_auths: readonly (readonly [string, number])[];
  },
  publicKey: string,
): boolean {
  const threshold = authority.weight_threshold;
  if (
    !Number.isSafeInteger(threshold) ||
    threshold < 1 ||
    threshold > 0xffffffff
  ) {
    return false;
  }

  const matchingKeys = authority.key_auths.filter(([key]) => key === publicKey);
  if (matchingKeys.length !== 1) {
    return false;
  }

  const weight = matchingKeys[0]![1];
  return (
    Number.isSafeInteger(weight) && weight >= threshold && weight <= 0xffff
  );
}

export interface WeightedMember {
  readonly id: string;
  readonly weight: number;
}
export interface AuthorityIssue {
  readonly kind:
    | 'zero-threshold'
    | 'no-members'
    | 'duplicate-member'
    | 'zero-weight-member'
    | 'threshold-unreachable'
    | 'lockout-on-member-loss';
  readonly error: boolean;
  readonly id?: string;
  readonly remainingWeight?: number;
}
export function analyzeAuthority(
  threshold: number,
  members: readonly WeightedMember[],
) {
  integer(threshold, 0, 0xffffffff);
  members.forEach((m) => integer(m.weight, 0, 0xffff));
  const issues: AuthorityIssue[] = [],
    seen = new Set<string>();
  if (!threshold) issues.push({ kind: 'zero-threshold', error: true });
  if (!members.length) issues.push({ kind: 'no-members', error: true });
  for (const m of members) {
    if (seen.has(m.id))
      issues.push({ kind: 'duplicate-member', error: true, id: m.id });
    seen.add(m.id);
    if (!m.weight)
      issues.push({ kind: 'zero-weight-member', error: false, id: m.id });
  }
  const totalWeight = members.reduce((s, m) => s + m.weight, 0);
  if (totalWeight < threshold)
    issues.push({ kind: 'threshold-unreachable', error: true });
  const reachable = !issues.some((i) => i.error) && threshold > 0,
    minimalSignerSets: string[][] = [];
  if (reachable && members.length > 1)
    for (const m of members)
      if (totalWeight - m.weight < threshold)
        issues.push({
          kind: 'lockout-on-member-loss',
          error: false,
          id: m.id,
          remainingWeight: totalWeight - m.weight,
        });
  if (reachable && members.length <= 12) {
    const masks: number[] = [];
    for (let mask = 1; mask < 1 << members.length; mask++) {
      const chosen = members.filter((_, i) => mask & (1 << i)),
        sum = chosen.reduce((s, m) => s + m.weight, 0);
      if (sum >= threshold && chosen.every((m) => sum - m.weight < threshold))
        masks.push(mask);
    }
    masks.sort((a, b) => popcount(a) - popcount(b) || a - b);
    for (const mask of masks.slice(0, 16))
      minimalSignerSets.push(
        members.filter((_, i) => mask & (1 << i)).map((m) => m.id),
      );
  }
  return { reachable, totalWeight, minimalSignerSets, issues };
}
function popcount(n: number): number {
  let count = 0;
  while (n) {
    n &= n - 1;
    count++;
  }
  return count;
}
export function ensureSufficientBalance(
  balance: bigint,
  amount: bigint,
  fee: bigint,
): void {
  if (balance < 0n || amount < 0n || fee < 0n || balance < amount + fee)
    throw new Error('Insufficient balance or negative amount');
}
