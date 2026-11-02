import {
  ActanetClient,
  type ActanetConnectionOptions,
} from '@open-graphene/chain-actanet-api';
import {
  BitSharesClient,
  type BitSharesConnectionOptions,
} from '@open-graphene/chain-bitshares-api';
import {
  RSquaredClient,
  type RSquaredConnectionOptions,
} from '@open-graphene/chain-rsquared-api';
export { ActanetClient, BitSharesClient, RSquaredClient };
export type {
  ActanetConnectionOptions,
  BitSharesConnectionOptions,
  RSquaredConnectionOptions,
};
export * as actanet from '@open-graphene/chain-actanet-api';
export * as bitshares from '@open-graphene/chain-bitshares-api';
export * as rsquared from '@open-graphene/chain-rsquared-api';
export * from '@open-graphene/core';
export class Graphene {
  static actanet(
    endpoints: string | readonly string[],
    options?: ActanetConnectionOptions,
  ) {
    return ActanetClient.connect(endpoints, options);
  }
  static bitshares(
    endpoints: string | readonly string[],
    options?: BitSharesConnectionOptions,
  ) {
    return BitSharesClient.connect(endpoints, options);
  }
  static rsquared(
    endpoints: string | readonly string[],
    options?: RSquaredConnectionOptions,
  ) {
    return RSquaredClient.connect(endpoints, options);
  }
  static connect(
    chain: 'actanet',
    endpoints: string | readonly string[],
    options?: ActanetConnectionOptions,
  ): Promise<ActanetClient>;
  static connect(
    chain: 'bitshares',
    endpoints: string | readonly string[],
    options?: BitSharesConnectionOptions,
  ): Promise<BitSharesClient>;
  static connect(
    chain: 'rsquared',
    endpoints: string | readonly string[],
    options?: RSquaredConnectionOptions,
  ): Promise<RSquaredClient>;
  static connect(
    chain: 'actanet' | 'bitshares' | 'rsquared',
    endpoints: string | readonly string[],
    options?: ActanetConnectionOptions,
  ) {
    if (chain === 'actanet') {
      return this.actanet(endpoints, options);
    }

    if (chain === 'bitshares') {
      return this.bitshares(endpoints, options);
    }

    if (chain === 'rsquared') {
      return this.rsquared(endpoints, options);
    }

    throw new Error('Unsupported chain');
  }
}

export * from '@open-graphene/fc/wallet';
export * from '@open-graphene/fc/memo';
export * as hash from '@open-graphene/fc/hash';
