import { runSwaplockSmoke } from '../tests/live/swaplock.mjs';

const args = process.argv.slice(2);
const browserMode = args.includes('--browser');
const endpoints = args.filter(arg => arg !== '--browser');
if (!endpoints.length) endpoints.push('wss://node01.swaplock.chainpool.online:8090', 'wss://node02.swaplock.chainpool.online:8090');
let browser;
let page;
try {
if (browserMode) {
  const { build } = await import('esbuild');
  const { chromium } = await import('@playwright/test');
  const { fileURLToPath } = await import('node:url');
  const bundle = await build({
    stdin: { contents: "import { runSwaplockSmoke } from './tests/live/swaplock.mjs'; globalThis.runSwaplockSmoke = runSwaplockSmoke;", resolveDir: fileURLToPath(new URL('../', import.meta.url)) },
    bundle: true, platform: 'browser', target: 'es2022', format: 'iife', write: false,
  });
  browser = await chromium.launch();
  page = await browser.newPage();
  await page.goto('https://portal.swaplock.chainpool.online/', { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.addScriptTag({ content: bundle.outputFiles[0].text });
}
for (const endpoint of endpoints) {
  try {
    const report = browserMode
      ? await page.evaluate(endpoint => globalThis.runSwaplockSmoke(endpoint), endpoint)
      : await runSwaplockSmoke(endpoint);
    report.runtime = browserMode ? 'Chromium (portal origin)' : 'Node.js';
    console.log(JSON.stringify(report, null, 2));
    if (report.checks.some(check => check.status === 'failed')) process.exitCode = 1;
  } catch (error) { console.error(`${endpoint}: ${error.message}`); process.exitCode = 1; }
}
} finally { await browser?.close(); }
