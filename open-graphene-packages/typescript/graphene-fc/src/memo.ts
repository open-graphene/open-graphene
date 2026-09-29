import { secp256k1 } from '@noble/curves/secp256k1.js';
import { sha256, sha512 } from '@noble/hashes/sha2.js';
import { bytesToHex } from '@open-graphene/primitives';
import { decodeWif } from './signing.js';

/** Graphene shared secret = SHA-512 of the ECDH point's padded 32-byte X coordinate. */
export function sharedSecret(privateKey: Uint8Array, publicKey: Uint8Array): Uint8Array {
 const point=secp256k1.getSharedSecret(privateKey,publicKey,false);
 try{return sha512(point.subarray(1,33));}finally{point.fill(0);}
}
function derive(privateKey:Uint8Array,publicKey:Uint8Array,nonce:bigint):Uint8Array{
 if(typeof nonce!=='bigint'||nonce<0n||nonce>0xffffffffffffffffn)throw new Error('Memo nonce must be uint64');
 const secret=sharedSecret(privateKey,publicKey);
 const seed=new TextEncoder().encode(nonce.toString()+bytesToHex(secret));
 try{return sha512(seed);}finally{secret.fill(0);seed.fill(0);}
}
export function uniqueNonce():bigint{
 const entropy=crypto.getRandomValues(new Uint8Array(8));
 return new DataView(entropy.buffer).getBigUint64(0,false);
}
export async function encryptMemo(privateKey:Uint8Array,publicKey:Uint8Array,nonce:bigint,message:Uint8Array):Promise<Uint8Array>{
 const material=derive(privateKey,publicKey,nonce);
 const payload=new Uint8Array(4+message.length);payload.set(sha256(message).subarray(0,4));payload.set(message,4);
 try{
  const key=await crypto.subtle.importKey('raw',material.slice(0,32),'AES-CBC',false,['encrypt']);
  return new Uint8Array(await crypto.subtle.encrypt({name:'AES-CBC',iv:material.slice(32,48)},key,payload));
 }finally{material.fill(0);payload.fill(0);}
}
export async function decryptMemo(privateKey:Uint8Array,publicKey:Uint8Array,nonce:bigint,ciphertext:Uint8Array):Promise<Uint8Array>{
 const material=derive(privateKey,publicKey,nonce);let plain:Uint8Array|undefined;
 try{
  const key=await crypto.subtle.importKey('raw',material.slice(0,32),'AES-CBC',false,['decrypt']);
  plain=new Uint8Array(await crypto.subtle.decrypt({name:'AES-CBC',iv:material.slice(32,48)},key,new Uint8Array(ciphertext)));
  if(plain.length<4)throw new Error();
  const checksum=sha256(plain.subarray(4));let diff=0;for(let i=0;i<4;i++)diff|=checksum[i]!^plain[i]!;
  if(diff!==0)throw new Error();
  return plain.slice(4);
 }catch{throw new Error('Memo decryption failed (key, nonce, checksum or padding)');}
 finally{material.fill(0);plain?.fill(0);}
}
export async function encryptMemoWithWif(wif:string,publicKey:Uint8Array,nonce:bigint,message:Uint8Array):Promise<Uint8Array>{
 const key=decodeWif(wif);try{return await encryptMemo(key,publicKey,nonce,message);}finally{key.fill(0);}
}
export async function decryptMemoWithWif(wif:string,publicKey:Uint8Array,nonce:bigint,ciphertext:Uint8Array):Promise<Uint8Array>{
 const key=decodeWif(wif);try{return await decryptMemo(key,publicKey,nonce,ciphertext);}finally{key.fill(0);}
}
