import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../', import.meta.url));
for (const chain of ['swaplock', 'bitshares']) {
  const family = `graphene-chain-${chain}`;
  const result = spawnSync('cargo', ['run', '--locked', '--offline', '-p',
    'open-graphene-gen-bindings-ts', '--', '--spec',
    `open-graphene-packages/rust/${family}/${family}-spec/dist/${chain}.open-graphene.json`,
    '--out-dir', `open-graphene-packages/typescript/${family}/${family}-bindings/src/generated`,
    ...process.argv.slice(2)], { cwd: root, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
