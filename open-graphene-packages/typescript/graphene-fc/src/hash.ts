import { sha256, sha512 } from '@noble/hashes/sha2.js';
import { hmac } from '@noble/hashes/hmac.js';
export { sha256, sha512 };
export { sha1, ripemd160 } from '@noble/hashes/legacy.js';
export const hmacSha256=(key:Uint8Array,data:Uint8Array)=>hmac(sha256,key,data);
export const hmacSha512=(key:Uint8Array,data:Uint8Array)=>hmac(sha512,key,data);
