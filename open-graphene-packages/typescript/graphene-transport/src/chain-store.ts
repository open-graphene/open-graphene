import { parseJson, stringifyJson, type WireValue } from '@open-graphene/codec';
import { RpcSubscription } from './subscription.js';
import type { GrapheneSession } from './session.js';
const clone=(value:WireValue)=>parseJson(stringifyJson(value));
export class ChainStore implements AsyncIterable<ReadonlyMap<string,WireValue>> {
 #objects=new Map<string,WireValue>();#ids:Set<string>;#closed=false;
 readonly changes:RpcSubscription<ReadonlyMap<string,WireValue>>;
 private constructor(private readonly notices:RpcSubscription,ids:readonly string[]){
  this.#ids=new Set(ids);this.changes=new RpcSubscription(0,()=>this.close());
 }
 static async create(client:GrapheneSession,ids:readonly string[]):Promise<ChainStore>{
  const notices=await client.databaseNotices(),store=new ChainStore(notices,ids);
  try{
   const initial=await client.request('database','get_objects',[ids,true]);
   store.#apply(initial);void store.#pump();return store;
  }catch(e){store.close();throw e;}
 }
 #apply(value:WireValue):void{
  if(Array.isArray(value)){for(const item of value)this.#apply(item);return;}
  if(typeof value==='string'){if(this.#ids.has(value))this.#objects.delete(value);return;}
  if(value&&typeof value==='object'&&'id' in value&&typeof value.id==='string'&&this.#ids.has(value.id))this.#objects.set(value.id,clone(value));
 }
 async #pump():Promise<void>{
  try{for await(const notice of this.notices){this.#apply(notice);this.changes.push(this.snapshot());}}
  catch(e){this.changes.fail(e instanceof Error?e:new Error(String(e)));}
  finally{this.close();}
 }
 get(id:string):WireValue|null{const value=this.#objects.get(id);return value===undefined?null:clone(value);}
 snapshot():ReadonlyMap<string,WireValue>{return new Map([...this.#objects].map(([id,v])=>[id,clone(v)]));}
 get size():number{return this.#objects.size;}
 close():void{if(this.#closed)return;this.#closed=true;this.notices.close();this.changes.close();}
 [Symbol.asyncIterator]():AsyncIterableIterator<ReadonlyMap<string,WireValue>>{return this.changes;}
}
