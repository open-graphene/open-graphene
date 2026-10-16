import {
  createChainAdapter,
  type ChainAdapter,
} from '@open-graphene/react-core';
import { CHAIN } from '@open-graphene/chain-rsquared-bindings';
import type { RSquaredClient } from '@open-graphene/chain-rsquared-api';
export const adapter: ChainAdapter<RSquaredClient> =
  createChainAdapter<RSquaredClient>(CHAIN.chainId);
