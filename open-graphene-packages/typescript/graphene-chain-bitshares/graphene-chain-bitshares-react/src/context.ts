import {
  createChainAdapter,
  type ChainAdapter,
} from '@open-graphene/react-core';
import { CHAIN } from '@open-graphene/chain-bitshares-bindings';
import type { BitSharesClient } from '@open-graphene/chain-bitshares-api';
export const adapter: ChainAdapter<BitSharesClient> =
  createChainAdapter<BitSharesClient>(CHAIN.chainId);
