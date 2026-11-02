// Copy the shared chain API source (graphene-chain-api/src) into every chain API package, under
// src/shared. Each package supplies src/bindings.ts, which the shared source imports as
// '../bindings.js'. With --check, fail instead of writing when a copy differs.
import { readdir, readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const source = root + 'graphene-chain-api/src/';
const check = process.argv.includes('--check');
const header =
  '// Generated from graphene-chain-api/src by scripts/generate-chain-api.mjs. Do not edit.\n';
const files = (await readdir(source)).filter((f) => f.endsWith('.ts')).sort();
let stale = false;
for (const chain of ['actanet', 'bitshares', 'rsquared']) {
  const target = `${root}graphene-chain-${chain}/graphene-chain-${chain}-api/src/shared/`;
  if (!check) {
    await rm(target, { recursive: true, force: true });
    await mkdir(target, { recursive: true });
  }
  for (const file of files) {
    const content = header + (await readFile(source + file, 'utf8'));
    if (check) {
      const current = await readFile(target + file, 'utf8').catch(() => '');
      if (current !== content) {
        console.error(`stale: graphene-chain-${chain}-api/src/shared/${file}`);
        stale = true;
      }
    } else {
      await writeFile(target + file, content);
    }
  }
  if (!check) console.log(`generated ${chain} api`);
}
if (stale) process.exit(1);
