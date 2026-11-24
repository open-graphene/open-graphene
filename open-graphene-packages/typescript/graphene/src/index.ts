import {
  MirrornetClient,
  type MirrornetConnectionOptions,
} from '@open-graphene/chain-mirrornet-api';
import {
  BitSharesClient,
  type BitSharesConnectionOptions,
} from '@open-graphene/chain-bitshares-api';
import {
  RSquaredClient,
  type RSquaredConnectionOptions,
} from '@open-graphene/chain-rsquared-api';
export { MirrornetClient, BitSharesClient, RSquaredClient };
export type {
  MirrornetConnectionOptions,
  BitSharesConnectionOptions,
  RSquaredConnectionOptions,
};
export * as mirrornet from '@open-graphene/chain-mirrornet-api';
export * as bitshares from '@open-graphene/chain-bitshares-api';
export * as rsquared from '@open-graphene/chain-rsquared-api';
export * from '@open-graphene/core';
export class Graphene {
  static mirrornet(
    endpoints: string | readonly string[],
    options?: MirrornetConnectionOptions,
  ) {
    return MirrornetClient.connect(endpoints, options);
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
    chain: 'mirrornet',
    endpoints: string | readonly string[],
    options?: MirrornetConnectionOptions,
  ): Promise<MirrornetClient>;
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
    chain: 'mirrornet' | 'bitshares' | 'rsquared',
    endpoints: string | readonly string[],
    options?: MirrornetConnectionOptions,
  ) {
    if (chain === 'mirrornet') {
      return this.mirrornet(endpoints, options);
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
