import {
  createChainAdapter,
  type ChainAdapter,
} from '@open-graphene/react-core';
import { CHAIN } from '@open-graphene/chain-actanet-bindings';
import type { ActanetClient } from '@open-graphene/chain-actanet-api';
export const adapter: ChainAdapter<ActanetClient> =
  createChainAdapter<ActanetClient>(CHAIN.chainId);
