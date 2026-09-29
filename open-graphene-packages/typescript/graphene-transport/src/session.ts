import { RpcClient, RpcTransportError, type ConnectionOptions } from './index.js';
import type { RpcMethod, WireValue } from '@open-graphene/codec';
import type { RpcSubscription } from './subscription.js';
export interface ReconnectPolicy { readonly maxRetries:number; readonly delayMs:number }
export interface SessionOptions extends ConnectionOptions { readonly expectedChainId?:string; readonly strategy?:'first-available'|'lowest-latency'; readonly reconnect?:ReconnectPolicy }
export interface ServerLatency { readonly endpoint:string;readonly milliseconds:number;readonly chainId:string }
export class GrapheneSession {
 #client:RpcClient;#closed=false;#reconnecting:Promise<void>|undefined;#policy:ReconnectPolicy;
 private constructor(client:RpcClient,readonly endpoint:string,readonly chainId:string,private readonly options:SessionOptions){
  this.#client=client;this.#policy=options.reconnect??{maxRetries:1,delayMs:100};this.setReconnectPolicy(this.#policy);
 }
 static async #open(endpoint:string,options:SessionOptions):Promise<GrapheneSession>{
  const client=await RpcClient.connect(endpoint,options);
  try{
   const id=await client.request('database','get_chain_id',[]);
   if(typeof id!=='string'||!/^[0-9a-f]{64}$/.test(id))throw new Error('Invalid chain ID');
   if(options.expectedChainId&&id!==options.expectedChainId)throw new Error('Connected node has a different chain ID');
   return new GrapheneSession(client,endpoint,id,options);
  }catch(e){client.close();throw e;}
 }
 static async connect(servers:string|readonly string[],options:SessionOptions={}):Promise<GrapheneSession>{
  const endpoints=typeof servers==='string'?[servers]:[...servers];if(!endpoints.length)throw new Error('No RPC endpoints');
  if(options.strategy==='lowest-latency'){
   const latencies=await this.probeLatencies(endpoints,options);
   endpoints.splice(0,endpoints.length,...latencies.map(l=>l.endpoint));
  }
  const errors:unknown[]=[];
  for(const endpoint of endpoints){try{return await this.#open(endpoint,options);}catch(e){errors.push(e);}}
  if(errors.length===1)throw errors[0];
  throw new AggregateError(errors,'No compatible RPC endpoint available');
 }
 static async probeLatencies(servers:readonly string[],options:SessionOptions={}):Promise<ServerLatency[]>{
  const results=await Promise.allSettled(servers.map(async endpoint=>{
   const start=performance.now(),session=await this.#open(endpoint,options);
   try{return {endpoint,milliseconds:performance.now()-start,chainId:session.chainId};}finally{session.close();}
  }));
  const ok=results.flatMap(r=>r.status==='fulfilled'?[r.value]:[]);
  if(!ok.length)throw new AggregateError(results.filter(r=>r.status==='rejected').map(r=>r.reason),'No compatible RPC endpoint available');
  return ok.sort((a,b)=>a.milliseconds-b.milliseconds);
 }
 setReconnectPolicy(policy:ReconnectPolicy):void{
  if(!Number.isSafeInteger(policy.maxRetries)||policy.maxRetries<0||policy.maxRetries>10||!Number.isSafeInteger(policy.delayMs)||policy.delayMs<0||policy.delayMs>60000)throw new Error('Invalid reconnect policy');
  this.#policy={...policy};
 }
 async reconnect():Promise<void>{
  if(this.#closed)throw new Error('Session closed');
  if(this.#reconnecting)return this.#reconnecting;
  this.#reconnecting=(async()=>{
   const next=await GrapheneSession.#open(this.endpoint,{...this.options,expectedChainId:this.chainId});
   if(this.#closed){next.close();throw new Error('Session closed');}
   this.#client.close();this.#client=next.#client;
  })();
  try{await this.#reconnecting;}finally{this.#reconnecting=undefined;}
 }
 request(api:string,method:string,args:readonly WireValue[]):Promise<WireValue>{
  if(this.#closed)return Promise.reject(new Error('Session closed'));
  // Raw requests, broadcasts and callback registration are never retried.
  return this.#client.request(api,method,args);
 }
 async invoke<P,R>(descriptor:RpcMethod<P,R>,params:P):Promise<R>{
  const read=/^(get_|lookup_|list_)/.test(descriptor.method);
  for(let attempt=0;;attempt++){
   try{return descriptor.parseReturns(await this.request(descriptor.api,descriptor.method,descriptor.encodeParams(params)));}
   catch(e){
    if(!read||!(e instanceof RpcTransportError)||attempt>=this.#policy.maxRetries||this.#closed)throw e;
    if(this.#policy.delayMs)await new Promise(resolve=>setTimeout(resolve,this.#policy.delayMs));
    await this.reconnect();
   }
  }
 }
 subscribe(api:string,method:string,args:(id:number)=>readonly WireValue[]):Promise<RpcSubscription>{return this.#client.subscribe(api,method,args);}
 databaseNotices():Promise<RpcSubscription>{return this.#client.databaseNotices();}
 close():void{this.#closed=true;this.#client.close();}
}
