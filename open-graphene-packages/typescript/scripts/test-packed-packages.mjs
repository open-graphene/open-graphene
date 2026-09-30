import { readdir, readFile, writeFile, mkdir, mkdtemp } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { join, basename } from 'node:path';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';
const root = fileURLToPath(new URL('../', import.meta.url));
const destination = process.argv[2] ?? await mkdtemp(join(tmpdir(), 'open-graphene-release-'));
await mkdir(destination, { recursive: true });
const packages = new Map();
for (const entry of await readdir(root, { withFileTypes: true })) {
  if (!entry.isDirectory() || !/^graphene/.test(entry.name)) continue;
  const folders = entry.name.startsWith('graphene-chain-')
    ? (await readdir(join(root, entry.name))).map(name => join(root, entry.name, name)) : [join(root, entry.name)];
  for (const folder of folders) {
    let manifest; try { manifest = JSON.parse(await readFile(join(folder, 'package.json'), 'utf8')); } catch (e) { if (e.code === 'ENOENT' || e.code === 'ENOTDIR') continue; throw e; }
    if (manifest.private) continue;
    packages.set(manifest.name, { folder, manifest });
  }
}
const order = [], visited = new Set(), active = new Set();
function visit(name) {
  if (visited.has(name)) return;
  if (active.has(name)) throw Error('Package dependency cycle: ' + name);
  active.add(name);
  const item = packages.get(name);
  for (const dependency of Object.keys(item.manifest.dependencies ?? {})) if (packages.has(dependency)) visit(dependency);
  active.delete(name); visited.add(name); order.push(name);
}
for (const name of [...packages.keys()].sort()) visit(name);
function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, encoding: 'utf8', env: { ...process.env, CI: 'true', npm_config_update_notifier: 'false' } });
  if (result.error) throw result.error;
  if (result.status !== 0) throw Error(`${command} failed: ${(result.stderr || result.stdout).slice(-5000)}`);
  return result.stdout;
}
const artifacts = [];
for (const name of order) {
  const item = packages.get(name);
  run('pnpm', ['pack', '--pack-destination', destination], item.folder);
  const tarball = join(destination, `${name.replace('@', '').replace('/', '-')}-${item.manifest.version}.tgz`);
  const manifest = JSON.parse(run('tar', ['-xOf', tarball, 'package/package.json'], root));
  const entries = run('tar', ['-tf', tarball], root).trim().split('\n');
  if (JSON.stringify(manifest).includes('workspace:')) throw Error('Unrewritten workspace dependency: ' + name);
  if (entries.some(entry => /(^|\/)(src|node_modules)\//.test(entry))) throw Error('Unwanted source/dependency files in ' + name);
  for (const license of ['LICENSE-MIT','LICENSE-APACHE']) if (!entries.includes('package/'+license)) throw Error('Missing license in '+name);
  for (const target of Object.values(manifest.exports)) for (const path of Object.values(target)) {
    if (!entries.includes('package/' + path.replace(/^\.\//, ''))) throw Error('Missing export ' + path + ' in ' + name);
  }
  artifacts.push({ name, version: manifest.version, file: basename(tarball), sha256: createHash('sha256').update(await readFile(tarball)).digest('hex') });
}
const overrides = Object.fromEntries(artifacts.map(item => [item.name, 'file:' + join(destination, item.file)]));
const consumerResults = [];
for (const chain of ['swaplock', 'bitshares']) {
  const consumer = join(destination, 'consumer-' + chain); await mkdir(consumer);
  const packageName = '@open-graphene/chain-' + chain + '-react';
  const peers = JSON.parse(await readFile(join(root, 'package.json'), 'utf8')).devDependencies;
  await writeFile(join(consumer, 'package.json'), JSON.stringify({ name: 'packed-' + chain + '-consumer', private: true, type: 'module', dependencies: {
    [packageName]: overrides[packageName], '@open-graphene/react-core': overrides['@open-graphene/react-core'],
    react: peers.react, '@types/react': '19.3.0', '@tanstack/react-query': peers['@tanstack/react-query'], typescript: peers.typescript,
  }, pnpm: { overrides } }, null, 2));
  const storeArgs = process.env.GRAPHENE_PNPM_STORE ? ['--store-dir', process.env.GRAPHENE_PNPM_STORE] : [];
  run('pnpm', ['install', '--offline', ...storeArgs], consumer);
  await writeFile(join(consumer, 'check.ts'), `
import { ${chain === 'swaplock' ? 'Swaplock' : 'BitShares'}Provider, useAccount, usePrepareLimitOrderCreate } from '${packageName}';
import { serializeCache } from '@open-graphene/react-core';
function typecheck() { const account = useAccount('alice', { live: true, select: a => a.name }); const name: string | undefined = account.data; void name; return usePrepareLimitOrderCreate({ maxFee: 1n }); }
void [typecheck, ${chain === 'swaplock' ? 'Swaplock' : 'BitShares'}Provider, serializeCache];
`);
  await writeFile(join(consumer, 'tsconfig.json'), JSON.stringify({ compilerOptions: { strict: true, noEmit: true, module: 'NodeNext', target: 'ES2022', skipLibCheck: false }, include: ['check.ts'] }));
  run('pnpm', ['exec', 'tsc', '-p', 'tsconfig.json'], consumer);
  const imports = run(process.execPath, ['--input-type=module', '-e', `const m = await import('${packageName}'); if(m.generatedPrepareOperations.length !== ${chain === 'swaplock' ? 88 : 71}) throw Error('Operation inventory'); const core = await import('@open-graphene/react-core'); if(!core.serializeCache({amount:9007199254740993n})) throw Error('Serialization'); console.log('ok');`], consumer).trim();
  if (imports !== 'ok') throw Error('Consumer import failed');
  const installed = run('pnpm', ['list', '--depth', 'Infinity', '--json'], consumer);
  if (installed.includes('@open-graphene/chain-' + (chain === 'swaplock' ? 'bitshares' : 'swaplock'))) throw Error('Cross-chain dependency in packed consumer');
  consumerResults.push({ chain, esm: 'passed', declarations: 'passed', isolation: 'passed' });
}
await writeFile(join(destination, 'release-plan.json'), JSON.stringify({ createdAt: new Date().toISOString(), publicationPerformed: false, order, artifacts, consumers: consumerResults }, null, 2) + '\n');
console.log(JSON.stringify({ packages: artifacts.length, consumers: consumerResults, releasePlan: join(destination, 'release-plan.json') }, null, 2));
