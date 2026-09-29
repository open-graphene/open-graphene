import { secp256k1 } from '@noble/curves/secp256k1.js';
import { base58 } from '@scure/base';
import { ripemd160 } from '@noble/hashes/legacy.js';
import { sha256 } from '@noble/hashes/sha2.js';

export interface Signer {
  readonly publicKey: Uint8Array;
  signDigest(digest: Uint8Array): Promise<Uint8Array>;
}
function digest32(digest: Uint8Array): void {
  if (!(digest instanceof Uint8Array) || digest.length !== 32) throw new Error('Expected a 32-byte digest');
}
export function encodePublicKey(key: Uint8Array, prefix: string): string {
  if (!/^[A-Z]+$/.test(prefix) || !secp256k1.utils.isValidPublicKey(key, true)) throw new Error('Invalid public key');
  const data = new Uint8Array(37); data.set(key); data.set(ripemd160(key).subarray(0, 4), 33);
  return prefix + base58.encode(data);
}
export function decodeWif(wif: string): Uint8Array {
  let decoded: Uint8Array | undefined;
  try {
    decoded = base58.decode(wif);
    if (decoded.length !== 37 || decoded[0] !== 0x80) throw new Error();
    const checksum = sha256(sha256(decoded.subarray(0, 33)));
    if (!checksum.subarray(0, 4).every((b, i) => b === decoded![i + 33])) throw new Error();
    const key = decoded.slice(1, 33);
    if (!secp256k1.utils.isValidSecretKey(key)) { key.fill(0); throw new Error(); }
    return key;
  } catch { throw new Error('Invalid Graphene WIF private key'); }
  finally { decoded?.fill(0); }
}
export type SigningProfile = 'swaplock-low-s' | 'graphene-legacy';
/** FC/bitsharesjs canonical compact integers: exactly 32 DER bytes for r and s. */
export function isCanonicalCompactSignature(signature: Uint8Array): boolean {
  return signature instanceof Uint8Array && signature.length === 65
    && signature[0]! >= 31 && signature[0]! <= 34
    && (signature[1]! & 0x80) === 0 && !(signature[1] === 0 && (signature[2]! & 0x80) === 0)
    && (signature[33]! & 0x80) === 0 && !(signature[33] === 0 && (signature[34]! & 0x80) === 0);
}
export function signDigestCompact(digest: Uint8Array, privateKey: Uint8Array, profile: SigningProfile = 'swaplock-low-s'): Uint8Array {
  digest32(digest);
  if (!secp256k1.utils.isValidSecretKey(privateKey)) throw new Error('Invalid private key');
  if (profile !== 'swaplock-low-s' && profile !== 'graphene-legacy') throw new Error('Unknown signing profile');
  // RFC6979 with deterministic extra entropy on retries. Signatures need not be
  // byte-identical to bitsharesjs; the signed digest and FC canonical rules are identical.
  for (let attempt = 0; attempt < 1024; attempt++) {
    const entropy = new Uint8Array(32);
    new DataView(entropy.buffer).setUint32(28, attempt, false);
    const result = secp256k1.sign(digest, privateKey, {
      prehash: false, lowS: true, format: 'recovered', extraEntropy: attempt === 0 ? false : entropy,
    });
    result[0] = result[0]! + 31;
    if (profile === 'swaplock-low-s' || isCanonicalCompactSignature(result)) return result;
  }
  throw new Error('Unable to produce a canonical compact signature');
}
export function recoverPublicKey(digest: Uint8Array, signature: Uint8Array): Uint8Array {
  digest32(digest);
  if (signature.length !== 65 || signature[0]! < 31 || signature[0]! > 34) throw new Error('Invalid compact signature header');
  const recovered = signature.slice(); recovered[0] = recovered[0]! - 31;
  const key = secp256k1.recoverPublicKey(recovered, digest, { prehash: false });
  if (!secp256k1.verify(recovered, digest, key, { prehash: false, lowS: true, format: 'recovered' })) throw new Error('Invalid low-S signature');
  return key;
}
export function verifyDigestCompact(digest: Uint8Array, signature: Uint8Array, key: Uint8Array): boolean {
  try { const recovered = recoverPublicKey(digest, signature); return recovered.length === key.length && recovered.every((b, i) => b === key[i]); }
  catch { return false; }
}
export class WifSigner implements Signer {
  #key: Uint8Array;
  #publicKey: Uint8Array;
  #disposed = false;
  constructor(wif: string, private readonly profile: SigningProfile = 'swaplock-low-s') { this.#key = decodeWif(wif); this.#publicKey = secp256k1.getPublicKey(this.#key, true); }
  get publicKey(): Uint8Array { return this.#publicKey.slice(); }
  async signDigest(digest: Uint8Array): Promise<Uint8Array> {
    if (this.#disposed) throw new Error('Signer disposed');
    return signDigestCompact(digest, this.#key, this.profile);
  }
  dispose(): void { this.#key.fill(0); this.#disposed = true; }
  toJSON(): object { return { type: 'WifSigner', disposed: this.#disposed }; }
}

export function privateKeyFromSeed(seed:Uint8Array):Uint8Array {
 const key=sha256(seed);
 if(!secp256k1.utils.isValidSecretKey(key)){key.fill(0);throw new Error('Seed produced invalid private key');}
 return key;
}
export function encodeWif(key:Uint8Array):string {
 if(!secp256k1.utils.isValidSecretKey(key))throw new Error('Invalid private key');
 const data=new Uint8Array(37);data[0]=0x80;data.set(key,1);data.set(sha256(sha256(data.subarray(0,33))).subarray(0,4),33);
 try{return base58.encode(data);}finally{data.fill(0);}
}
export function publicKeyFromPrivateKey(key:Uint8Array):Uint8Array{return secp256k1.getPublicKey(key,true);}
