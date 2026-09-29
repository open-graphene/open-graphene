import * as b from '@open-graphene/chain-swaplock-bindings';
import { sha256, decodePublicKey } from '@open-graphene/fc';
import { bytesToHex, parseObjectId } from '@open-graphene/primitives';
export const DATA_ROOM_PERM_UPDATE_ROOM=1,DATA_ROOM_PERM_ADD_MEMBERS=2,DATA_ROOM_PERM_REMOVE_MEMBERS=4,
 DATA_ROOM_PERM_MANAGE_PERMISSIONS=8,DATA_ROOM_PERM_CREATE_CONTENT=16,DATA_ROOM_PERM_MANAGE_CONTENT=32,
 DATA_ROOM_PERM_ROTATE_KEYS=64,DATA_ROOM_PERM_GRANT_CONTENT=128,DATA_ROOM_PERM_ALL=255;
export function memberRef(value:string):b.DataRoomMemberRef{
 if(value.startsWith(b.CHAIN.publicKeyPrefix)){decodePublicKey(value,b.CHAIN.publicKeyPrefix);return [1,value as b.PublicKey];}
 if(value.startsWith('1.23.'))return [2,b.DataRoomId(value)];
 return [0,b.AccountId(value)];
}
export function roomAccessStateDigest(state:b.DataRoomAccessState):Uint8Array{
 if(state.domain!=='swaplock:data-room-access:v1')throw new Error('Unknown room access domain');
 for(let i=1;i<state.members.length;i++)if(state.members[i-1]!.membership_instance>=state.members[i]!.membership_instance)throw new Error('Members must have strictly increasing instances');
 return sha256(b.encodeDataRoomAccessState(state));
}
export class RoomAccessPrecondition {
 private constructor(readonly room:b.DataRoomId,readonly digest:string){Object.freeze(this);}
 static fromDigest(room:string,digest:string):RoomAccessPrecondition{
  parseObjectId(room,1,23);if(!/^[0-9a-f]{64}$/.test(digest))throw new Error('Expected lowercase SHA-256 digest');
  return new RoomAccessPrecondition(b.DataRoomId(room),digest);
 }
 static fromSnapshot(state:b.DataRoomAccessState):RoomAccessPrecondition{return this.fromDigest(state.room,bytesToHex(roomAccessStateDigest(state)));}
 guard(operation:b.Operation):b.Operation{
  const op=b.OperationCodec.decode(b.OperationCodec.encode(operation));
  const guardedTags = [
   b.operation.data_room_member_add({} as b.DataRoomMemberAddOperation)[0],
   b.operation.data_room_member_update({} as b.DataRoomMemberUpdateOperation)[0],
   b.operation.data_room_member_remove({} as b.DataRoomMemberRemoveOperation)[0],
   b.operation.data_room_rotate_key({} as b.DataRoomRotateKeyOperation)[0],
  ];
  if(!guardedTags.some(tag=>tag===op[0]))throw new Error('Operation does not support access precondition');
  return guardRoomOperation(op,this.room,this.digest);
 }
}
function guardRoomOperation(op:b.Operation,room:b.DataRoomId,digest:string):b.Operation{
 const payload=op[1] as unknown as {room?:string;extensions?:{expected_access_state?:string}};
 if(payload.room!==room)throw new Error('Access precondition belongs to another room');
 if(payload.extensions?.expected_access_state!==undefined&&payload.extensions.expected_access_state!==digest)throw new Error('Different access precondition already attached');
 payload.extensions={...payload.extensions,expected_access_state:digest};return op;
}

const coreFee=()=>({amount:0n,asset_id:b.AssetId('1.3.0')});
function compareMembers(a:b.DataRoomMemberRef,c:b.DataRoomMemberRef):number{
 if(a[0]!==c[0])return a[0]-c[0];
 if(a[0]===1&&c[0]===1){
  const x=decodePublicKey(a[1],b.CHAIN.publicKeyPrefix),y=decodePublicKey(c[1],b.CHAIN.publicKeyPrefix);
  for(let i=0;i<x.length;i++){if(x[i]!==y[i])return x[i]!-y[i]!;}return 0;
 }
 const x=parseObjectId(a[1]).instance,y=parseObjectId(c[1]).instance;return x<y?-1:x>y?1:0;
}
export function memberAddOperation(caller:string,room:string,member:string,memberKey:string,epochKeys:readonly (readonly [number,string])[]=[],permissions=0):b.Operation{
 if(!Number.isSafeInteger(permissions)||permissions<0||permissions>DATA_ROOM_PERM_ALL)throw new Error('Unknown room permission flags');
 const epochs=[...epochKeys].sort((a,c)=>a[0]-c[0]);
 if(epochs.some((v,i)=>i>0&&epochs[i-1]![0]===v[0]))throw new Error('Duplicate epoch key');
 return b.operation.data_room_member_add({fee:coreFee(),caller:b.AccountId(caller),room:b.DataRoomId(room),member:memberRef(member),member_key:memberKey,epoch_keys:epochs,permissions,extensions:{}});
}
export function memberRemoveOperation(caller:string,room:string,member:string):b.Operation{
 return b.operation.data_room_member_remove({fee:coreFee(),caller:b.AccountId(caller),room:b.DataRoomId(room),member:memberRef(member),extensions:{}});
}
export function rotateKeyOperation(caller:string,room:string,memberKeys:readonly (readonly [string,string])[]):b.Operation{
 if(!memberKeys.length)throw new Error('Member keys required');
 const keys=memberKeys.map(([member,key])=>[memberRef(member),key] as const).sort((a,c)=>compareMembers(a[0],c[0]));
 if(keys.some((v,i)=>i>0&&compareMembers(keys[i-1]![0],v[0])===0))throw new Error('Duplicate member key');
 return b.operation.data_room_rotate_key({fee:coreFee(),caller:b.AccountId(caller),room:b.DataRoomId(room),member_keys:keys,extensions:{}});
}
export function contentCardGrantCreateOperation(granter:string,contentId:string,grantee:string,key:string):b.Operation{
 if(!key)throw new Error('Grant key required');
 return b.operation.content_card_grant_create({fee:coreFee(),granter:b.AccountId(granter),content_id:b.ContentCardId(contentId),grantee:memberRef(grantee),key,extensions:[]});
}
