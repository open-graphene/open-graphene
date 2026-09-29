import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { testAllMethods } from '../tests/live/all-methods.mjs';
const args=process.argv.slice(2);
const browserMode=args.includes('--browser');
const [output='/tmp/swaplock-all-methods.json',fixturePath]=args.filter(a=>a!=='--browser');
const fixture=fixturePath?JSON.parse(await readFile(fixturePath,'utf8')):{};
const options={marketAsset:fixture.assetId,subjectAsset:fixture.assetId,subjectRoomId:fixture.subjectRoomId,cardId:fixture.grantCardId};
const runs=[];
let browser,page;
try {
 if(browserMode){
  const {build}=await import('esbuild');
  const {chromium}=await import('@playwright/test');
  const bundle=await build({stdin:{contents:"import {testAllMethods} from './tests/live/all-methods.mjs'; globalThis.testAllMethods=testAllMethods;",resolveDir:fileURLToPath(new URL('../',import.meta.url))},bundle:true,platform:'browser',target:'es2022',format:'iife',write:false});
  browser=await chromium.launch();page=await browser.newPage();
  await page.goto('https://portal.swaplock.chainpool.online/',{waitUntil:'domcontentloaded',timeout:30000});
  await page.addScriptTag({content:bundle.outputFiles[0].text});
 }
 for(const endpoint of ['wss://node01.swaplock.chainpool.online:8090','wss://node02.swaplock.chainpool.online:8090']){
  const report=browserMode?await page.evaluate(({endpoint,options})=>globalThis.testAllMethods(endpoint,options),{endpoint,options}):await testAllMethods(endpoint,options);
  report.runtime=browserMode?'Chromium (portal origin)':'Node.js';runs.push(report);
  console.log(JSON.stringify({endpoint,runtime:report.runtime,summary:report.summary,failures:report.methods.filter(m=>m.status==='failed')},null,2));
  if(report.methods.some(m=>['failed','not_tested','blocked_by_test_data'].includes(m.status))||report.scenarios.some(s=>s.status==='failed'))process.exitCode=1;
 }
}finally{
 await browser?.close();
 await writeFile(output,JSON.stringify({runs},null,2)+'\n');
}
