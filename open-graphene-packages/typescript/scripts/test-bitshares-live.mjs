import {writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {runBitSharesSmoke} from '../tests/live/bitshares.mjs';
import {stringifyJson} from '../graphene-codec/dist/index.js';
const args=process.argv.slice(2),browserMode=args.includes('--browser');
const [output='/tmp/bitshares-sdk-live.json',...configured]=args.filter(x=>x!=='--browser');
const endpoints=configured.length?configured:['wss://api.dex.trading/','wss://public.xbts.io/ws'];
const runs=[];let browser,page;
try{
 if(browserMode){
  const {build}=await import('esbuild');const {chromium}=await import('@playwright/test');
  const bundle=await build({stdin:{contents:"import {runBitSharesSmoke} from './tests/live/bitshares.mjs'; import {stringifyJson} from './graphene-codec/dist/index.js'; globalThis.runSmoke=async endpoint=>stringifyJson(await runBitSharesSmoke(endpoint));",resolveDir:fileURLToPath(new URL('../',import.meta.url))},bundle:true,platform:'browser',target:'es2022',format:'iife',write:false});
  browser=await chromium.launch();page=await browser.newPage();await page.addScriptTag({content:bundle.outputFiles[0].text});
 }
 for(const endpoint of endpoints){
  try{
   const report=browserMode?JSON.parse(await page.evaluate(e=>globalThis.runSmoke(e),endpoint)):await runBitSharesSmoke(endpoint);
   report.runtime=browserMode?'Chromium':'Node.js';runs.push(report);
   console.log(endpoint,report.runtime,JSON.stringify(report.summary));
   if(report.checks.some(c=>c.status==='failed'))process.exitCode=1;
  }catch(error){runs.push({endpoint,runtime:browserMode?'Chromium':'Node.js',status:'failed',reason:error.message});process.exitCode=1;console.error(error.message);}
 }
}finally{await browser?.close();await writeFile(output,stringifyJson({runs})+'\n');}
