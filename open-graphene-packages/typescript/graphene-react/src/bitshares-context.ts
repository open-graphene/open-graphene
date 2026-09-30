import { createChainAdapter } from './adapter.js';
import { CHAIN } from '@open-graphene/chain-bitshares-bindings';
import type { BitSharesClient } from '@open-graphene/chain-bitshares-api';
export const adapter = createChainAdapter<BitSharesClient>(CHAIN.chainId);
