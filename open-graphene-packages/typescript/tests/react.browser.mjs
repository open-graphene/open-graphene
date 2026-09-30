import { createElement as h, StrictMode, act } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SwaplockProvider, useAccount, useDynamicGlobalProperties, useBroadcastTransaction, useWaitForInclusion, useDatabaseGetChainId, accountOptions } from '../graphene-react/dist/swaplock.js';
import { BitSharesProvider, useAccount as useBitSharesAccount, useDynamicGlobalProperties as useBitSharesHead } from '../graphene-react/dist/bitshares.js';
import { CHAIN } from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { CHAIN as BTS } from '../graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';
import { RpcSubscription, RpcTransportError } from '../graphene-transport/dist/index.js';
globalThis.IS_REACT_ACT_ENVIRONMENT = true;
const assert = (value, message) => { if (!value) throw Error(message); };
const pause = () => new Promise(resolve => setTimeout(resolve, 5));
async function settle() { await act(async () => { await pause(); }); }
function fake(chainId = CHAIN.chainId) {
  const streams = [],counts={reads:0,opens:0,closes:0,reconnects:0,sends:0,inclusions:0};
  const client={chainId,
    rpc:{invoke:async()=>chainId},
    database:{account:async name=>{counts.reads++;return {name,id:'1.2.100'};},
      watchAccount:async name=>{counts.opens++;const s=new RpcSubscription(1,()=>counts.closes++);streams.push(s);s.push({name,id:'1.2.100'});return s;}},
    reconnect:async()=>{counts.reconnects++;},
    broadcast:async()=>{counts.sends++;throw new RpcTransportError('Ambiguous broadcast',true);},
    waitForInclusion:async signed=>{counts.inclusions++;return {transactionId:signed.id,blockNumber:1};},
  };
  return {client,streams,counts};
}
export async function runReactChecks() {
  const q=new QueryClient({defaultOptions:{queries:{gcTime:Infinity},mutations:{retry:3}}});
  const element=document.createElement('div');document.body.append(element);const root=createRoot(element);
  const a=fake(),b=fake();let latest={},mutation,inclusion;
  function Account({label,name='alice',enabled=true}) {
    const result=useAccount(name,{live:true,enabled,select:data=>data.name});latest[label]=result;
    return h('span',{'data-label':label},result.data??'pending');
  }
  function Commands(){mutation=useBroadcastTransaction();inclusion=useWaitForInclusion();return null;}
  const wrap=children=>h(StrictMode,null,h(QueryClientProvider,{client:q},children));
  const tree=(client,children)=>wrap(h(SwaplockProvider,{client,livePolicy:{delayMs:0,maxRetries:2}},children));
  try {
    await act(async()=>root.render(tree(a.client,h('div',null,h(Account,{label:'one'}),h(Account,{label:'two'})))));await settle();
    assert(a.counts.opens===1,'Strict Mode/concurrent consumers opened duplicate streams');
    assert(latest.one.data==='alice'&&latest.two.data==='alice','initial snapshot/selection');
    await act(async()=>a.streams[0].push({name:'updated',id:'1.2.100'}));await settle();
    assert(latest.one.data==='updated'&&latest.two.data==='updated','shared update');
    await act(async()=>root.render(tree(a.client,h(Account,{label:'one'}))));await settle();
    assert(a.counts.closes===0,'stream closed while a consumer remained');
    await act(async()=>a.streams[0].fail(new RpcTransportError('closed',true)));await settle();await settle();
    assert(a.counts.reconnects===1&&a.counts.opens===2,'reconnect and resubscribe');
    assert(latest.one.live.status==='live','live status after fresh snapshot');
    await act(async()=>root.render(tree(a.client,h(Account,{label:'one',name:'bob'}))));await settle();
    assert(latest.one.data==='bob'&&a.counts.closes===2,'parameter change retains old subscription/data');
    await act(async()=>root.render(tree(a.client,h(Account,{label:'one',name:'bob',enabled:false}))));await settle();
    assert(a.counts.closes===3&&latest.one.live.status==='disabled','disabled hook retains stream');
    await act(async()=>root.render(tree(b.client,h(Account,{label:'one'}))));await settle();
    assert(b.counts.opens===1&&latest.one.data==='alice','provider client replacement');
    await act(async()=>root.render(tree(b.client,h(Commands))));await settle();
    await act(async()=>{await mutation.mutateAsync({id:'public-tx'}).catch(()=>{});});await settle();
    assert(b.counts.sends===1,'broadcast retried despite global mutation retry setting');
    const key=accountOptions(b.client,'alice').queryKey;q.setQueryData(key,{name:'before-inclusion'});
    await act(async()=>{await inclusion.mutateAsync({id:'public-tx'});});await settle();
    assert(q.getQueryState(key).isInvalidated,'inclusion did not invalidate scoped reads');
    const bt=fake(BTS.chainId);
    function BitAccount(){const result=useBitSharesAccount('bit-account');latest.bit=result;return null;}
    function Generated(){latest.chain=useDatabaseGetChainId({});return null;}
    await act(async()=>root.render(wrap(h('div',null,
      h(SwaplockProvider,{client:b.client},h(Generated)),
      h(BitSharesProvider,{client:bt.client},h(BitAccount))))));await settle();
    assert(latest.chain.data===CHAIN.chainId&&latest.bit.data.name==='bit-account','generated reads/chain providers');
    return {strictMode:true,sharedStreams:true,parameterSwitch:true,disable:true,clientSwitch:true,reconnect:true,noBroadcastRetry:true,scopedInvalidation:true,generatedQueries:true,bitshares:true};
  } finally {await act(async()=>root.unmount());q.clear();element.remove();}
}

// Optional read-only live test; no wallet or broadcast code runs here.
export async function runLiveReact(client, chain = 'swaplock') {
  const q=new QueryClient(),element=document.createElement('div');document.body.append(element);const root=createRoot(element);
  const heads=new Set();let status;
  const Provider = chain === 'swaplock' ? SwaplockProvider : BitSharesProvider;
  const useHead = chain === 'swaplock' ? useDynamicGlobalProperties : useBitSharesHead;
  function Head(){const result=useHead({live:true});if(result.data)heads.add(result.data.head_block_number);status=result.live;return null;}
  try {
    await act(async()=>root.render(h(QueryClientProvider,{client:q},h(Provider,{client},h(Head)))));
    for(let i=0;i<200&&heads.size<2;i++)await act(async()=>{await new Promise(r=>setTimeout(r,100));});
    assert(heads.size>=2&&status.status==='live','live React did not observe two blocks');
    return {chainId:client.chainId,heads:[...heads],status:status.status};
  }finally{await act(async()=>root.unmount());q.clear();element.remove();}
}
