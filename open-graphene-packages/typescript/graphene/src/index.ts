import { SwaplockClient } from '@open-graphene/chain-swaplock-api';
import { BitSharesClient } from '@open-graphene/chain-bitshares-api';
import type { SessionOptions } from '@open-graphene/transport';
export { SwaplockClient, BitSharesClient };
export * as swaplock from '@open-graphene/chain-swaplock-api';
export * as bitshares from '@open-graphene/chain-bitshares-api';
export * from '@open-graphene/core';
export class Graphene {
 static swaplock(endpoints:string|readonly string[],options?:SessionOptions){return SwaplockClient.connect(endpoints,options);}
 static bitshares(endpoints:string|readonly string[],options?:SessionOptions){return BitSharesClient.connect(endpoints,options);}
 static connect(chain:'swaplock',endpoints:string|readonly string[],options?:SessionOptions):Promise<SwaplockClient>;
 static connect(chain:'bitshares',endpoints:string|readonly string[],options?:SessionOptions):Promise<BitSharesClient>;
 static connect(chain:'swaplock'|'bitshares',endpoints:string|readonly string[],options?:SessionOptions){
  if(chain==='swaplock')return this.swaplock(endpoints,options);
  if(chain==='bitshares')return this.bitshares(endpoints,options);
  throw new Error('Unsupported chain');
 }
}

export * from '@open-graphene/fc/wallet';
export * from '@open-graphene/fc/memo';
export * as hash from '@open-graphene/fc/hash';
