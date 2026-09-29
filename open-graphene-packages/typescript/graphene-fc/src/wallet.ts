import { secp256k1 } from '@noble/curves/secp256k1.js';
import { sha256, sha512 } from '@noble/hashes/sha2.js';
import { ripemd160 } from '@noble/hashes/legacy.js';
import { base58 } from '@scure/base';
import { bytesToHex, hexToBytes, integer } from '@open-graphene/primitives';
import { decodePublicKey } from './index.js';
import { decodeWif, encodeWif, privateKeyFromSeed, signDigestCompact, recoverPublicKey, verifyDigestCompact, type Signer, type SigningProfile } from './signing.js';
import { sharedSecret } from './memo.js';
export class PublicKey {
 #bytes:Uint8Array;
 private constructor(bytes:Uint8Array){
  if(bytes.length!==33||(!bytes.every(b=>b===0)&&!secp256k1.utils.isValidPublicKey(bytes,true)))throw new Error('Invalid public key');
  this.#bytes=bytes.slice();
 }
 static fromBytes(bytes:Uint8Array){return new PublicKey(bytes);}
 static fromHex(hex:string){return this.fromBytes(hexToBytes(hex,33));}
 static fromString(value:string,prefix='BTS'){return this.fromBytes(decodePublicKey(value,prefix));}
 static null(){return this.fromBytes(new Uint8Array(33));}
 static recover(digest:Uint8Array,signature:Uint8Array){return this.fromBytes(recoverPublicKey(digest,signature));}
 get bytes(){return this.#bytes.slice();}
 toString(prefix='BTS'):string{
  if(!/^[A-Z]+$/.test(prefix))throw new Error('Invalid public key prefix');
  const payload=new Uint8Array(37);payload.set(this.#bytes);payload.set(ripemd160(this.#bytes).subarray(0,4),33);return prefix+base58.encode(payload);
 }
 verify(digest:Uint8Array,signature:Uint8Array){return verifyDigestCompact(digest,signature,this.#bytes);}
}
export class PrivateKey implements Signer {
 #bytes:Uint8Array;#disposed=false;
 private constructor(bytes:Uint8Array,private readonly profile:SigningProfile){
  if(!secp256k1.utils.isValidSecretKey(bytes))throw new Error('Invalid private key');this.#bytes=bytes.slice();
 }
 static fromBytes(bytes:Uint8Array,profile:SigningProfile='swaplock-low-s'){return new PrivateKey(bytes,profile);}
 static fromWif(wif:string,profile:SigningProfile='swaplock-low-s'){
  const key=decodeWif(wif);try{return this.fromBytes(key,profile);}finally{key.fill(0);}
 }
 static fromSeed(seed:Uint8Array,profile:SigningProfile='swaplock-low-s'){
  const key=privateKeyFromSeed(seed);try{return this.fromBytes(key,profile);}finally{key.fill(0);}
 }
 #alive(){if(this.#disposed)throw new Error('Private key disposed');}
 get bytes(){this.#alive();return this.#bytes.slice();}
 get publicKey(){this.#alive();return secp256k1.getPublicKey(this.#bytes,true);}
 toPublicKey(){return PublicKey.fromBytes(this.publicKey);}
 toWif(){this.#alive();return encodeWif(this.#bytes);}
 async signDigest(digest:Uint8Array){this.#alive();return signDigestCompact(digest,this.#bytes,this.profile);}
 getSharedSecret(peer:PublicKey){this.#alive();return sharedSecret(this.#bytes,peer.bytes);}
 dispose(){this.#bytes.fill(0);this.#disposed=true;}
 toJSON(){return {type:'PrivateKey',disposed:this.#disposed};}
}
export const SUGGESTED_BRAIN_KEY_WORDS=16;
export function normalizeBrainKey(value:string):string{return value.split(/[ \t\n\v\f\r]+/).filter(Boolean).join(' ');}
export class BrainKey {
 #words:Uint8Array;#disposed=false;
 constructor(passphrase:string){this.#words=new TextEncoder().encode(normalizeBrainKey(passphrase));}
 static suggest(dictionary:string,wordCount=SUGGESTED_BRAIN_KEY_WORDS){
  integer(wordCount,0,1024);
  const words=dictionary.split(/[, \t\n\v\f\r]+/).filter(Boolean);
  if(!words.length||words.length>0xffffffff)throw new Error('Invalid brain-key dictionary');
  const limit=0x100000000-(0x100000000%words.length),chosen:string[]=[];
  for(let i=0;i<wordCount;i++){let n:number;do{n=crypto.getRandomValues(new Uint32Array(1))[0]!;}while(n>=limit);chosen.push(words[n%words.length]!);}
  return new BrainKey(chosen.join(' '));
 }
 get words(){if(this.#disposed)throw new Error('Brain key disposed');return new TextDecoder().decode(this.#words);}
 privateKey(sequence=0,profile:SigningProfile='swaplock-low-s'){
  integer(sequence,0,0xffffffff);const seed=new TextEncoder().encode(this.words+' '+sequence);
  const key=sha256(sha512(seed));try{return PrivateKey.fromBytes(key,profile);}finally{seed.fill(0);key.fill(0);}
 }
 dispose(){this.#words.fill(0);this.#disposed=true;}
 toJSON(){return {type:'BrainKey',disposed:this.#disposed};}
}
export function accountRoleKey(account:string,password:string,role:string,profile:SigningProfile='swaplock-low-s'){
 const seed=new TextEncoder().encode(normalizeBrainKey(account+role+password));
 try{return PrivateKey.fromSeed(seed,profile);}finally{seed.fill(0);}
}
export class AccountKeys {
 private constructor(readonly owner:PrivateKey,readonly active:PrivateKey,readonly memo:PrivateKey){}
 static derive(account:string,password:string,profile:SigningProfile='swaplock-low-s'){
  return new AccountKeys(accountRoleKey(account,password,'owner',profile),accountRoleKey(account,password,'active',profile),accountRoleKey(account,password,'memo',profile));
 }
 dispose(){this.owner.dispose();this.active.dispose();this.memo.dispose();}
 toJSON(){return {type:'AccountKeys'};}
}
export class Address {
 #bytes:Uint8Array;
 private constructor(bytes:Uint8Array){if(bytes.length!==20)throw new Error('Invalid address length');this.#bytes=bytes.slice();}
 static fromPublicKey(key:PublicKey){return new Address(ripemd160(sha512(key.bytes)));}
 static fromBytes(bytes:Uint8Array){return new Address(bytes);}
 static fromString(value:string,prefix='BTS'){
  if(!value.startsWith(prefix))throw new Error('Address prefix mismatch');
  const bytes=base58.decode(value.slice(prefix.length));
  if(bytes.length!==24||!ripemd160(bytes.subarray(0,20)).subarray(0,4).every((v,i)=>v===bytes[i+20]))throw new Error('Invalid address checksum');
  return new Address(bytes.subarray(0,20));
 }
 get bytes(){return this.#bytes.slice();}
 toString(prefix='BTS'){
  if(!/^[A-Z]+$/.test(prefix))throw new Error('Invalid address prefix');
  const payload=new Uint8Array(24);payload.set(this.#bytes);payload.set(ripemd160(this.#bytes).subarray(0,4),20);return prefix+base58.encode(payload);
 }
}
export class Signature {
 #bytes:Uint8Array;
 constructor(bytes:Uint8Array){if(bytes.length!==65||bytes[0]!<31||bytes[0]!>34)throw new Error('Invalid compact signature');this.#bytes=bytes.slice();}
 static fromHex(hex:string){return new Signature(hexToBytes(hex,65));}
 get bytes(){return this.#bytes.slice();}
 toHex(){return bytesToHex(this.#bytes);}
 recover(digest:Uint8Array){return PublicKey.recover(digest,this.#bytes);}
 verify(digest:Uint8Array,key:PublicKey){return key.verify(digest,this.#bytes);}
}
