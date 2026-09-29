import * as b from '../../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { SwaplockClient } from '../../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import { RpcRemoteError } from '../../graphene-transport/dist/index.js';
import { bytesToHex } from '../../graphene-primitives/dist/index.js';
const expect = (v, m) => { if (!v) throw new Error(m); };
const shape = v => v === null ? { kind: 'null' } : v instanceof Uint8Array ? { kind: 'bytes', length: v.length } : Array.isArray(v) ? { kind: 'array', length: v.length } : { kind: typeof v };
export async function testAllMethods(endpoint, { marketAsset, subjectAsset, subjectRoomId, cardId = '1.26.0' } = {}) {
  const report = { endpoint, startedAt: new Date().toISOString(), methods: Object.values(b.rpc).flatMap(api => Object.values(api)).map(d => ({ method: `${d.api}.${d.method}`, status: 'not_tested' })), scenarios: [] };
  const client = await SwaplockClient.connect(endpoint);
  const row = d => report.methods.find(r => r.method === `${d.api}.${d.method}`);
  const call = async (d, params = {}, verify = () => {}) => {
    try { const v = await client.rpc.invoke(d, params); verify(v); Object.assign(row(d), { status: 'passed', result: shape(v) }); return v; }
    catch(e) { const reason = e.detail?.message ?? e.message; Object.assign(row(d), { status: /Access denied|not enabled|not available/i.test(reason) ? 'unavailable_on_node' : 'failed', reason: reason.slice(0,500) }); }
  };
  const blocked = (d, reason) => Object.assign(row(d), { status: 'blocked_by_test_data', reason });
  const scenario = async (name, action) => { try { await action(); report.scenarios.push({name,status:'passed'}); } catch(e) {report.scenarios.push({name,status:'failed',reason:e.message.slice(0,500)});} };
  try {
    report.chainId = await call(b.DatabaseGetChainId, {}, id => expect(id === b.CHAIN.chainId, 'Chain mismatch'));
    const head = await call(b.DatabaseGetDynamicGlobalProperties); report.headBlock = head?.head_block_number;
    for(const d of [b.DatabaseGetChainProperties,b.DatabaseGetGlobalProperties,b.DatabaseGetConfig]) await call(d);
    const assets = await call(b.DatabaseListAssets,{lower_bound_symbol:'',limit:100}, a=>expect(a.length>0,'No assets'));
    const quote=assets?.find(a=>a.id!=='1.3.0'&&(!marketAsset||a.id===marketAsset||a.symbol===marketAsset));
    report.market = quote ? {base:'1.3.0',quote:quote.id} : null;
    await call(b.DatabaseGetAssets,{asset_symbols_or_ids:['1.3.0','1.3.281474976710655'],subscribe:false},a=>expect(a[0]?.id==='1.3.0'&&a[1]===null,'Asset/null slots'));
    await call(b.DatabaseLookupAssetSymbols,{symbols_or_ids:['BTS','1.3.0']},a=>expect(a.length===2&&a.every(x=>x?.id==='1.3.0'),'Asset lookup'));
    await call(b.DatabaseLookupAccounts,{lower_bound_name:'',limit:3,subscribe:false},a=>expect(a.length>0,'Empty account lookup'));
    const accounts=await call(b.DatabaseGetAccounts,{account_names_or_ids:['swaplock','registrar','1.2.281474976710655'],subscribe:false},a=>expect(a[0]?.name==='swaplock'&&a[1]?.name==='registrar'&&a[2]===null,'Account/null slots'));
    await call(b.DatabaseGetFullAccounts,{names_or_ids:['swaplock'],subscribe:false},a=>expect(a.length===1&&a[0][1].account.name==='swaplock','Full account'));
    await call(b.DatabaseGetAccountBalances,{account_name_or_id:'swaplock',assets:['1.3.0']},a=>expect(a.length===1&&typeof a[0].amount==='bigint','Balance type'));
    if(accounts?.[0])await call(b.DatabaseGetKeyReferences,{keys:[accounts[0].active.key_auths[0][0]]},a=>expect(a[0].includes(accounts[0].id),'Key reference'));
    await call(b.DatabaseGetObjects,{ids:['1.2.100','1.3.0','2.1.0','1.2.281474976710655'],subscribe:false},a=>expect(a[0].kind==='account'&&a[3]===null,'Object routing'));
    await call(b.DatabaseGetBlockHeader,{block_num:head?.head_block_number??1201598},v=>expect(v!==null,'Missing header'));
    await call(b.DatabaseGetBlock,{block_num:1201598},v=>expect(v?.transactions.length>0,'Missing transaction-bearing block'));
    const op=b.operation.transfer(b.TransferOperationCodec.decode({from:'1.2.100',to:'1.2.101',amount:{amount:1,asset_id:'1.3.0'},fee:{amount:0,asset_id:'1.3.0'},extensions:[]}));
    await call(b.DatabaseGetRequiredFees,{ops:[op],asset_symbol_or_id:'1.3.0'},v=>expect(v[0].amount>0n,'Fee'));
    await call(b.HistoryGetAccountHistory,{account_name_or_id:'swaplock',stop:'1.11.0',limit:10,start:'1.11.0'},v=>expect(v.length>0,'Empty account history'));
    const groups=await call(b.OrdersGetTrackedGroups,{},v=>expect(v.length>0,'Empty groups'));
    if(quote){
      await call(b.DatabaseGetLimitOrders,{a:'1.3.0',b:quote.id,limit:10});
      await call(b.DatabaseGetTicker,{base:'1.3.0',quote:quote.id});
      await call(b.HistoryGetFillOrderHistory,{a:'1.3.0',b:quote.id,limit:10});
      await call(b.HistoryGetMarketHistory,{a:'1.3.0',b:quote.id,bucket_seconds:60,start:new Date(Date.now()-86400000).toISOString().slice(0,19),end:new Date(Date.now()+3600000).toISOString().slice(0,19)});
      await call(b.OrdersGetGroupedLimitOrders,{base_asset:quote.id,quote_asset:'1.3.0',group:groups?.[0]??10,start:null,limit:10});
    }else for(const d of [b.DatabaseGetLimitOrders,b.DatabaseGetTicker,b.HistoryGetFillOrderHistory,b.HistoryGetMarketHistory,b.OrdersGetGroupedLimitOrders])blocked(d,'Distinct market asset required; only BTS exists');
    const room=await call(b.DatabaseGetDataRoomById,{room_id:'1.23.0'},v=>expect(v?.id==='1.23.0','Missing room fixture'));
    await call(b.DatabaseGetDataRoomAccessState,{room_id:'1.23.0'},v=>expect(v!==null,'Missing access state'));
    const members=await call(b.DatabaseGetDataRoomMembers,{room_id:'1.23.0',limit:10});
    const member=members?.[0]?.member[1]??room?.owner??'1.2.100';
    await call(b.DatabaseGetDataRoomsByOwner,{account_name_or_id:room?.owner??'1.2.100',limit:100},a=>expect(a.some(x=>x.id==='1.23.0'),'Owner listing'));
    const subject=subjectAsset??(room?.subject[0]!==0?room?.subject[1]:'1.2.100');
    await call(b.DatabaseGetDataRoomsBySubject,{asset_or_account:subject,limit:100},a=>{if(subjectRoomId)expect(a.some(x=>x.id===subjectRoomId),'Subject listing');});
    await call(b.DatabaseGetDataRoomMember,{room_id:'1.23.0',member_name_key_or_id:member},v=>{if(members?.length)expect(v?.id===members[0].id,'Member lookup');});
    await call(b.DatabaseGetDataRoomsByMember,{member_name_key_or_id:member,limit:100});
    const epochs=await call(b.DatabaseGetDataRoomKeyEpochs,{room_id:'1.23.0',member_name_key_or_id:member,limit:10});
    await call(b.DatabaseGetDataRoomKeyEpoch,{room_id:'1.23.0',epoch:epochs?.[0]?.epoch??room?.current_epoch??0,member_name_key_or_id:member});
    const card=await call(b.DatabaseGetContentCardById,{content_id:cardId},v=>expect(v?.id===cardId,'Missing card fixture'));
    await call(b.DatabaseGetContentCardsByRoom,{room_id:card?.room??'1.23.0',limit:100},a=>expect(a.some(x=>x.id===cardId),'Room card listing'));
    await call(b.DatabaseGetContentCardsByAuthor,{author_name_key_or_id:card?.author[1]??member,limit:100},a=>expect(a.some(x=>x.id===cardId),'Author card listing'));
    const grants=await call(b.DatabaseGetContentCardGrantsByCard,{content_id:cardId,limit:10},v=>expect(v.length>0,'Missing grant fixture'));
    const grantee=grants?.[0]?.grantee[1]??member;
    await call(b.DatabaseGetContentCardGrant,{content_id:cardId,grantee_name_key_or_id:grantee},v=>{if(grants?.length)expect(v?.id===grants[0].id,'Grant lookup');});
    await call(b.DatabaseGetContentCardGrantsByGrantee,{grantee_name_key_or_id:grantee,limit:100});
    const blind=new Uint8Array(32);blind[31]=1;const nonce=new Uint8Array(32).fill(2);
    const commit=await call(b.CryptoBlind,{blind,value:7n},v=>expect(v.length===33,'Commitment length'));
    await call(b.CryptoBlindSum,{blinds_in:[blind,blind],non_neg:1},v=>expect(v.every(x=>x===0),'Blind subtraction'));
    const commitHex=bytesToHex(commit??new Uint8Array(33));
    await call(b.CryptoVerifySum,{commits_in:[commitHex],neg_commits_in:[commitHex],excess:0n},v=>expect(v===true,'Commitment balance'));
    const proof=await call(b.CryptoRangeProofSign,{min_value:0n,commit:commitHex,commit_blind:blind,nonce,base10_exp:0,min_bits:8,actual_value:7n},v=>expect(v.length>0,'Empty proof'));
    await call(b.CryptoVerifyRange,{commit:commit??new Uint8Array(33),proof:proof??new Uint8Array()},v=>expect(v.success&&v.min_val<=7n&&v.max_val>=7n,'Range verification'));
    await call(b.CryptoVerifyRangeProofRewind,{nonce,commit:commit??new Uint8Array(33),proof:proof??new Uint8Array()},v=>expect(v.success&&v.value_out===7n&&bytesToHex(v.blind_out)===bytesToHex(blind),'Proof rewind'));
    await call(b.CryptoRangeGetInfo,{proof:proof??new Uint8Array()},v=>expect(v.min_value<=7n&&v.max_value>=7n,'Proof info'));
    if(commit && proof) {
      report.cryptoEvidence = { value: '7', blindHex: bytesToHex(blind), nonceHex: bytesToHex(nonce), commitmentHex: commitHex, proofHex: bytesToHex(proof) };
      await scenario('crypto rejects unbalanced commitment sum', async()=>{
        expect(await client.rpc.invoke(b.CryptoVerifySum,{commits_in:[commitHex],neg_commits_in:[commitHex],excess:1n})===false,'Unbalanced sum accepted');
      });
      await scenario('crypto rejects corrupted range proof', async()=>{
        const corrupted=proof.slice();corrupted[corrupted.length-1]^=1;
        const result=await client.rpc.invoke(b.CryptoVerifyRange,{commit,proof:corrupted});
        expect(result.success===false,'Corrupted proof accepted');
      });
      await scenario('crypto rejects wrong rewind nonce', async()=>{
        const wrongNonce=nonce.slice();wrongNonce[0]^=1;
        try {
          const result=await client.rpc.invoke(b.CryptoVerifyRangeProofRewind,{nonce:wrongNonce,commit,proof});
          expect(result.success===false,'Wrong nonce accepted');
          report.cryptoEvidence.wrongNonceRejection='success=false';
        } catch(error) {
          // FC_ASSERT in the native rewind implementation rejects an invalid nonce.
          if(!(error instanceof RpcRemoteError) || !/Assert Exception: secp256k1_rangeproof_rewind\(/.test(error.detail?.message??'')) throw error;
          report.cryptoEvidence.wrongNonceRejection='native secp256k1_rangeproof_rewind assertion (RPC error)';
        }
      });
      await scenario('crypto preserves value above Number.MAX_SAFE_INTEGER', async()=>{
        const value=9007199254740993n;
        const wideCommit=await client.rpc.invoke(b.CryptoBlind,{blind,value});
        const wideProof=await client.rpc.invoke(b.CryptoRangeProofSign,{min_value:0n,commit:bytesToHex(wideCommit),commit_blind:blind,nonce,base10_exp:0,min_bits:64,actual_value:value});
        const verification=await client.rpc.invoke(b.CryptoVerifyRange,{commit:wideCommit,proof:wideProof});
        expect(verification.success&&verification.min_val<=value&&verification.max_val>=value,'Wide range verification');
        const rewound=await client.rpc.invoke(b.CryptoVerifyRangeProofRewind,{nonce,commit:wideCommit,proof:wideProof});
        expect(rewound.success&&rewound.value_out===value&&bytesToHex(rewound.blind_out)===bytesToHex(blind),'Wide value lost precision');
        report.cryptoEvidence.wideValue=rewound.value_out.toString();
        report.cryptoEvidence.wideCommitmentHex=bytesToHex(wideCommit);
      });
      await scenario('crypto proof verifies on the other RPC node', async()=>{
        const peerEndpoint=endpoint.includes('node01.')?endpoint.replace('node01.','node02.'):endpoint.replace('node02.','node01.');
        const peer=await SwaplockClient.connect(peerEndpoint);
        try {
          const result=await peer.rpc.invoke(b.CryptoVerifyRange,{commit,proof});
          expect(result.success&&result.min_val<=7n&&result.max_val>=7n,'Cross-node proof verification');
        } finally { peer.close(); }
      });
    }
    Object.assign(row(b.NetworkBroadcastBroadcastTransaction),{status:'separate_live_transfer_scenario'});
    for(const [name,request,message] of [
      ['fee cap',{from:'swaplock',to:'registrar',amount:1n,maxFee:0n},'exceeds maximum fee'],
      ['missing recipient',{from:'swaplock',to:'1.2.281474976710655',amount:1n,maxFee:300000n},'does not exist']
    ])await scenario(name,async()=>{let error;try{await client.prepareTransfer(request)}catch(e){error=e}expect(error?.message.includes(message),'Expected rejection');});
    await scenario('missing room/card/block are null',async()=>{
      expect(await client.rpc.invoke(b.DatabaseGetDataRoomById,{room_id:'1.23.281474976710655'})===null,'Room');
      expect(await client.rpc.invoke(b.DatabaseGetContentCardById,{content_id:'1.26.281474976710655'})===null,'Card');
      expect(await client.rpc.invoke(b.DatabaseGetBlock,{block_num:0xffffffff})===null,'Block');
    });
  } finally {client.close();report.finishedAt=new Date().toISOString();}
  expect(report.methods.length===47,'SDK inventory changed');
  report.summary=Object.fromEntries([...new Set(report.methods.map(r=>r.status))].map(s=>[s,report.methods.filter(r=>r.status===s).length]));
  return report;
}
