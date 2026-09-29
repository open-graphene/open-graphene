import { FcWriter, decodePublicKey } from './index.js';
import { parseObjectId, parseVoteId, parseTimePointSec, hexToBytes } from '@open-graphene/primitives';

interface Type { readonly [key: string]: unknown; readonly kind: string; readonly name?: string; readonly objectType?: string; readonly inner?: Type; readonly key?: Type; readonly value?: Type; readonly first?: Type; readonly second?: Type; readonly bytes?: number; readonly chainPrefix?: string; readonly json?: string }
interface Field { readonly name: string; readonly type: Type; readonly index: number }
interface Definition { readonly kind: string; readonly fields?: readonly Field[]; readonly extension?: boolean; readonly arms?: readonly { readonly tag: number; readonly type: Type; readonly virtual?: boolean }[] }
export interface FcSchema { readonly prefix: string; readonly objects: Readonly<Record<string, readonly number[]>>; readonly definitions: Readonly<Record<string, Definition>> }
type Value = unknown;
type SortKey = bigint | Uint8Array | readonly SortKey[];
function list(value: Value): readonly Value[] { if (!Array.isArray(value)) throw new Error('Expected FC array'); return value; }
function record(value: Value): Record<string, Value> { if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Expected FC struct'); return value as Record<string,Value>; }
function num(value: Value): bigint { if(typeof value!=='bigint' && !(typeof value==='number'&&Number.isSafeInteger(value))) throw new Error('Expected exact FC integer'); return BigInt(value); }
function text(value: Value): string { if(typeof value!=='string')throw new Error('Expected FC string');return value; }
function bytes(value: Value): Uint8Array { if(!(value instanceof Uint8Array))throw new Error('Expected FC bytes');return value; }
function compare(a: SortKey,b: SortKey): number {
 if(typeof a==='bigint'&&typeof b==='bigint')return a<b?-1:a>b?1:0;
 if(a instanceof Uint8Array&&b instanceof Uint8Array){for(let i=0;i<Math.min(a.length,b.length);i++){if(a[i]!==b[i])return a[i]!<b[i]!?-1:1;}return a.length-b.length;}
 if(Array.isArray(a)&&Array.isArray(b)){for(let i=0;i<Math.min(a.length,b.length);i++){const n=compare(a[i]!,b[i]!);if(n)return n;}return a.length-b.length;}
 throw new Error('Incomparable FC keys');
}
function sortKey(schema: FcSchema,t: Type,v: Value): SortKey {
 switch(t.kind){
 case 'bool':return v===true?1n:0n;
 case 'uint8':case 'uint16':case 'uint32':case 'int32':case 'int64':case 'uint64':case 'unsigned_varint':return num(v);
 case 'protocol_object_id':return parseObjectId(text(v)).instance;
 case 'object_id':{const id=parseObjectId(text(v));return (BigInt(id.space)<<56n)|(BigInt(id.type)<<48n)|id.instance;}
 case 'vote_id':return BigInt(parseVoteId(text(v)));
 case 'string':return new TextEncoder().encode(text(v));
 case 'time_point_sec':return BigInt(parseTimePointSec(text(v)));
 case 'public_key':return decodePublicKey(v,t.chainPrefix??schema.prefix);
 case 'fixed_bytes':return bytes(v);
 case 'void':return new Uint8Array();
 case 'static_variant_ref':{const [tag,payload]=list(v);const arm=schema.definitions[t.name!]?.arms?.find(a=>BigInt(a.tag)===num(tag));if(!arm)throw new Error('Invalid FC variant');return [num(tag),sortKey(schema,arm.type,payload)];}
 default:throw new Error('Unsupported FC ordering: '+t.kind);
 }
}
function assertOrdered(schema:FcSchema,t:Type,values:readonly Value[],tagOnly=false):void{
 let previous:SortKey|undefined;
 for(const value of values){const key=tagOnly?num(list(value)[0]):sortKey(schema,t,value);if(previous!==undefined&&compare(previous,key)>=0)throw new Error('FC set/map must be sorted and unique');previous=key;}
}
const sparseSupported=new Set(['content_card_update_operation_ext','content_card_remove_operation_ext','data_room_access_extensions','data_room_create_operation_ext']);
export function encodeFc(schema: FcSchema,name:string,value:unknown):Uint8Array {
 const w=new FcWriter();
 function definition(name:string,value:Value,depth:number):void{
  const def=schema.definitions[name];if(!def)throw new Error('Unknown FC definition '+name);
  if(def.kind==='variant'){
   const [tag,payload]=list(value),arm=def.arms?.find(a=>BigInt(a.tag)===num(tag));
   if(!arm)throw new Error('Unknown FC variant tag');
   if(arm.virtual)throw new Error('Virtual operations cannot be signed');
   w.varint(num(tag));write(arm.type,payload,depth+1);return;
  }
  const obj=record(value),fields=def.fields??[];
  if(def.extension){
   const present=fields.filter(f=>obj[f.name]!=null);
   if(present.length&&!sparseSupported.has(name))throw new Error('Nonempty extension not supported by Rust FC profile: '+name);
   w.varint(present.length);
   for(const f of present){if(f.type.kind!=='optional')throw new Error('Invalid extension schema');w.varint(f.index);write(f.type.inner!,obj[f.name],depth+1);}
  }else for(const f of fields)write(f.type,obj[f.name],depth+1);
 }
 function write(t:Type,v:Value,depth:number):void{
  if(depth>128)throw new Error('FC nesting limit exceeded');
  switch(t.kind){
   case 'void':return;
   case 'bool':if(typeof v!=='boolean')throw new Error('Expected bool');w.uint(v?1:0,1);return;
   case 'uint8':case 'uint16':case 'uint32':case 'uint64':case 'uint128':w.uint(num(v),Number(t.kind.slice(4))/8);return;
   case 'int32':case 'int64':w.int(num(v),Number(t.kind.slice(3))/8);return;
   case 'unsigned_varint':w.varint(num(v));return;
   case 'string':w.string(text(v));return;
   case 'bytes':{const b=bytes(v);w.varint(b.length);w.bytes(b);return;}
   case 'fixed_bytes':w.bytes(bytes(v),t.bytes);return;
   case 'fixed_hex':w.bytes(hexToBytes(text(v),t.bytes),t.bytes);return;
   case 'signature':w.bytes(bytes(v),65);return;
   case 'time_point_sec':w.time(text(v));return;
   case 'vote_id':w.vote(text(v));return;
   case 'public_key':w.bytes(decodePublicKey(v,t.chainPrefix??schema.prefix),33);return;
   case 'object_id':w.objectId(text(v));return;
   case 'protocol_object_id':{const ids=schema.objects[t.objectType!];if(!ids)throw new Error('Unknown FC object type');w.typedId(text(v),ids[0]!,ids[1]!);return;}
   case 'ref':case 'static_variant_ref':definition(t.name!,v,depth);return;
   case 'optional':w.uint(v==null?0:1,1);if(v!=null)write(t.inner!,v,depth+1);return;
   case 'vector':case 'set':{
    const items=list(v);
    if(t.kind==='set')assertOrdered(schema,t.inner!,items,t.inner!.kind==='static_variant_ref'&&['fee_parameters','future_extensions'].includes(t.inner!.name!));
    w.varint(items.length);for(const item of items)write(t.inner!,item,depth+1);return;
   }
   case 'pair':{const pair=list(v);if(pair.length!==2)throw new Error('Invalid pair');write(t.first!,pair[0],depth+1);write(t.second!,pair[1],depth+1);return;}
   case 'flat_map':case 'map':{
    const pairs=list(v).map(list);
    if(t.key!.kind==='address'){if(pairs.length)throw new Error('Nonempty address map unsupported by Rust FC profile');w.varint(0);return;}
    assertOrdered(schema,t.key!,pairs.map(p=>p[0]));
    w.varint(pairs.length);for(const p of pairs){write(t.key!,p[0],depth+1);write(t.value!,p[1],depth+1);}return;
   }
   default:throw new Error('Unsupported FC type '+t.kind);
  }
 }
 definition(name,value,0);return w.finish();
}
