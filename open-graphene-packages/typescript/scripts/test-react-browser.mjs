import { build } from 'esbuild';
import { chromium } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import { writeFile } from 'node:fs/promises';
const live=process.argv.includes('--live');
const bundle=await build({stdin:{contents:`
 import {runReactChecks,runLiveReact} from './tests/react.browser.mjs';
 import {SwaplockClient} from './graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
 import {BitSharesClient} from './graphene-chain-bitshares/graphene-chain-bitshares-api/dist/index.js';
 globalThis.runReactChecks=runReactChecks;
 globalThis.runLiveReact=async({chain,endpoint})=>{const c=await (chain==='swaplock'?SwaplockClient:BitSharesClient).connect(endpoint);try{return {chain,endpoint,...await runLiveReact(c,chain)};}finally{c.close();}};
 `,resolveDir:fileURLToPath(new URL('../',import.meta.url)),sourcefile:'react-browser-test.js'},bundle:true,platform:'browser',format:'iife',write:false,define:{'process.env.NODE_ENV':'"development"'}});
const browser=await chromium.launch({headless:true});
try{
 const page=await browser.newPage();const errors=[];
 page.on('pageerror',error=>errors.push(error.message));
 await page.route('https://sdk.test/',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>React SDK tests</title>'}));
 await page.goto('https://sdk.test/');await page.addScriptTag({content:bundle.outputFiles[0].text});
 console.log('React Chromium:',await page.evaluate(()=>globalThis.runReactChecks()));
 if(live){
  const runs=[];
  for(const [chain,endpoint] of [['swaplock','wss://node01.swaplock.chainpool.online:8090'],['swaplock','wss://node02.swaplock.chainpool.online:8090'],['bitshares','wss://api.dex.trading/'],['bitshares','wss://public.xbts.io/ws']]){
   const result=await page.evaluate(input=>globalThis.runLiveReact(input),{chain,endpoint});runs.push(result);console.log('React live:',result);
  }
  await writeFile('/tmp/open-graphene-react-live.json',JSON.stringify({date:new Date().toISOString(),broadcast:false,runs},null,2)+'\n');
 }
 if(errors.length)throw Error(errors.join('\n'));
}finally{await browser.close();}
