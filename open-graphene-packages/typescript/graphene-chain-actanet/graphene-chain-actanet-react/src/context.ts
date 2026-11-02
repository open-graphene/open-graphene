import {
  createChainAdapter,
  type ChainAdapter,
} from '@open-graphene/react-core';
import { CHAIN } from '@open-graphene/chain-actanet-bindings';
import { ActanetClient } from '@open-graphene/chain-actanet-api';
export const adapter: ChainAdapter<ActanetClient> =
  // Any network of the chain: a client of this chain's API, or one with the bundled chain ID
  createChainAdapter<ActanetClient>(
    (client: { readonly chainId: string }) =>
      client instanceof ActanetClient || client.chainId === CHAIN.chainId,
  );
