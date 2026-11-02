import {
  createChainAdapter,
  type ChainAdapter,
} from '@open-graphene/react-core';
import { CHAIN } from '@open-graphene/chain-bitshares-bindings';
import { BitSharesClient } from '@open-graphene/chain-bitshares-api';
export const adapter: ChainAdapter<BitSharesClient> =
  // Any network of the chain: a client of this chain's API, or one with the bundled chain ID
  createChainAdapter<BitSharesClient>(
    (client: { readonly chainId: string }) =>
      client instanceof BitSharesClient || client.chainId === CHAIN.chainId,
  );
