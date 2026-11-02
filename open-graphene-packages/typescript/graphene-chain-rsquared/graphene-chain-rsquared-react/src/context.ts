import {
  createChainAdapter,
  type ChainAdapter,
} from '@open-graphene/react-core';
import { CHAIN } from '@open-graphene/chain-rsquared-bindings';
import { RSquaredClient } from '@open-graphene/chain-rsquared-api';
export const adapter: ChainAdapter<RSquaredClient> =
  // Any network of the chain: a client of this chain's API, or one with the bundled chain ID
  createChainAdapter<RSquaredClient>(
    (client: { readonly chainId: string }) =>
      client instanceof RSquaredClient || client.chainId === CHAIN.chainId,
  );
