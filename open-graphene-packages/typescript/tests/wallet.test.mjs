import test from 'node:test';
import assert from 'node:assert/strict';
import {PublicKey,PrivateKey,BrainKey,AccountKeys,Address,Signature,normalizeBrainKey} from '../graphene-fc/dist/wallet.js';
import {sha256,hmacSha256} from '../graphene-fc/dist/hash.js';
import {bytesToHex} from '../graphene-primitives/dist/index.js';
const text=s=>new TextEncoder().encode(s);
test('account login matches Rust reference and disposes derived keys',()=>{
 const keys=AccountKeys.derive('someaccountname','somereallylongpassword');
 assert.equal(keys.active.toPublicKey().toString('GPH'),'GPH5Abm5dCdy3hJ1C5ckXkqUH2Me7dXqi9Y7yjn9ACaiSJ9h8r8mL');
 assert.notDeepEqual(keys.active.publicKey,keys.owner.publicKey);
 assert.equal(JSON.stringify(keys),'{"type":"AccountKeys"}');keys.dispose();assert.throws(()=>keys.active.toWif(),/disposed/);
});
test('wallet key, address and signature round trips validate checksums',async()=>{
 const key=PrivateKey.fromSeed(text('public test seed')),pub=key.toPublicKey(),digest=sha256(text('message'));
 assert.deepEqual(PrivateKey.fromWif(key.toWif()).publicKey,pub.bytes);
 assert.deepEqual(PublicKey.fromString(pub.toString()).bytes,pub.bytes);
 const address=Address.fromPublicKey(pub),encoded=address.toString();assert.deepEqual(Address.fromString(encoded).bytes,address.bytes);
 assert.throws(()=>Address.fromString(encoded.slice(0,-1)+(encoded.endsWith('1')?'2':'1')),/checksum/);
 const signature=new Signature(await key.signDigest(digest));assert.ok(signature.verify(digest,pub));assert.deepEqual(signature.recover(digest).bytes,pub.bytes);
 assert.equal(signature.verify(sha256(text('wrong')),pub),false);
 assert.deepEqual(PublicKey.fromString(PublicKey.null().toString()).bytes,new Uint8Array(33));
 key.dispose();await assert.rejects(key.signDigest(digest),/disposed/);
});
test('brain keys normalize whitespace, derive sequences and redact secrets',()=>{
 assert.equal(normalizeBrainKey(' ONE\t TWO\nTHREE '),'ONE TWO THREE');
 const a=new BrainKey(' ONE\t TWO '),b=new BrainKey('ONE TWO');
 assert.deepEqual(a.privateKey().publicKey,b.privateKey().publicKey);assert.notDeepEqual(a.privateKey().publicKey,a.privateKey(1).publicKey);
 assert.equal(BrainKey.suggest('ONE,TWO,THREE').words.split(' ').length,16);
 assert.throws(()=>BrainKey.suggest(''));assert.throws(()=>a.privateKey(-1));assert.ok(!JSON.stringify(a).includes('ONE'));a.dispose();assert.throws(()=>a.words,/disposed/);
});
test('hash and HMAC match published standard vectors',()=>{
 assert.equal(bytesToHex(sha256(text('abc'))),'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
 assert.equal(bytesToHex(hmacSha256(new Uint8Array(20).fill(11),text('Hi There'))),'b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7');
});
