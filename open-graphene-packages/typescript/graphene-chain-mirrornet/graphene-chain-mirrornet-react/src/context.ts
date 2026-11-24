import {
  createChainAdapter,
  type ChainAdapter,
} from '@open-graphene/react-core';
import { CHAIN } from '@open-graphene/chain-mirrornet-bindings';
import { MirrornetClient } from '@open-graphene/chain-mirrornet-api';
export const adapter: ChainAdapter<MirrornetClient> =
  // Any network of the chain: a client of this chain's API, or one with the bundled chain ID
  createChainAdapter<MirrornetClient>(
    (client: { readonly chainId: string }) =>
      client instanceof MirrornetClient || client.chainId === CHAIN.chainId,
  );
