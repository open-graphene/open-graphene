// Explicit opt-in: broadcasts creation/deletion on the pinned Swaplock testnet.
import { readFile, writeFile, rename, rm } from 'node:fs/promises';
import { withReactDom } from './react-dom-harness.mjs';
const [genesisPath, reportPath = '/tmp/open-graphene-react-room-live.json'] = process.argv.slice(2);
if (!genesisPath) throw Error('Pass a local genesis file path to authorize the Swaplock room E2E');
const result = await withReactDom(19, `
 export {runRoomLifecycle} from './tests/react-room-lifecycle.mjs';
 export {SwaplockClient} from './graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
 export {PrivateKey} from './graphene-fc/dist/wallet.js';
`, async module => {
  // Connect before loading secret material; this also checks the pinned chain identity.
  const client = await module.SwaplockClient.connect('wss://node01.swaplock.chainpool.online:8090', { timeoutMs: 5000 });
  let signer;
  try {
    const genesis = JSON.parse(await readFile(genesisPath, 'utf8'));
    const record = genesis.initial_accounts.find(account => account.name === 'swaplock');
    if (!record?.active_key_full?.wif_priv_key) throw Error('Genesis has no swaplock active key');
    signer = module.PrivateKey.fromWif(record.active_key_full.wif_priv_key);
    const owner = await client.database.account('swaplock');
    const publicKey = signer.toPublicKey().toString('BTS');
    const weight = owner.active.key_auths.find(([key]) => key === publicKey)?.[1] ?? 0;
    if (weight < owner.active.weight_threshold) throw Error('Genesis key no longer satisfies the account active authority');
    let journal;
    try { journal = JSON.parse(await readFile(reportPath, 'utf8')); }
    catch (error) { if (error.code !== 'ENOENT') throw error; journal = { chainId: client.chainId, startedAt: new Date().toISOString(), roomName: 'react-e2e-' + Date.now().toString(36), transactions: [], checks: [] }; }
    if (journal.chainId !== client.chainId) throw Error('Journal chain ID mismatch');
    const save = async () => {
      const temporary = reportPath + '.' + process.pid + '.tmp';
      try {
        await writeFile(temporary, JSON.stringify(journal, null, 2) + '\n', { mode: 0o600 });
        await rename(temporary, reportPath);
      } finally { await rm(temporary, { force: true }); }
    };
    return await module.runRoomLifecycle({ client, signer, owner, journal, save });
  } finally { signer?.dispose(); client.close(); }
});
console.log('React room E2E:', result);
