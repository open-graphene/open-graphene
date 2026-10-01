import {
  createChainAdapter,
  type ChainAdapter,
} from '@open-graphene/react-core';
import { CHAIN } from '@open-graphene/chain-swaplock-bindings';
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
export const adapter: ChainAdapter<SwaplockClient> =
  createChainAdapter<SwaplockClient>(CHAIN.chainId);
