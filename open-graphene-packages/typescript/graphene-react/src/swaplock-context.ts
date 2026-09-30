import { createChainAdapter } from './adapter.js';
import { CHAIN } from '@open-graphene/chain-swaplock-bindings';
import type { SwaplockClient } from '@open-graphene/chain-swaplock-api';
export const adapter = createChainAdapter<SwaplockClient>(CHAIN.chainId);
