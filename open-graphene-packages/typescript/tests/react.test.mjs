import test from 'node:test';
import assert from 'node:assert/strict';
import { QueryClient, dehydrate, hydrate } from '@tanstack/react-query';
import {
  serializeCache,
  deserializeCache,
  grapheneQueryKey,
} from '@open-graphene/react-core';
import { LiveEntry, sharedLive } from '../graphene-react-core/dist/live.js';
import {
  RpcSubscription,
  RpcTransportError,
  RpcRemoteError,
} from '../graphene-transport/dist/index.js';
import * as swaplock from '@open-graphene/chain-swaplock-react';
import * as bitshares from '@open-graphene/chain-bitshares-react';
import { CHAIN } from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { readFileSync } from 'node:fs';
import * as swaplockBindings from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import * as bitsharesBindings from '../graphene-chain-bitshares/graphene-chain-bitshares-bindings/dist/index.js';
import { bytesToHex } from '../graphene-primitives/dist/index.js';
const pause = () => new Promise((resolve) => setTimeout(resolve, 0));
async function until(fn) {
  for (let i = 0; i < 100; i++) {
    if (fn()) return;
    await pause();
  }
  assert.fail('condition did not become true');
}
const cache = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } },
  });
test('React query keys isolate clients, chains and tagged values; hydrate bigint/bytes exactly', () => {
  const a = { chainId: 'a' },
    b = { chainId: 'a' },
    c = { chainId: 'b' };
  assert.notDeepEqual(
    grapheneQueryKey(a, 'x', []),
    grapheneQueryKey(b, 'x', []),
  );
  assert.notDeepEqual(
    grapheneQueryKey(a, 'x', [], 'shared'),
    grapheneQueryKey(c, 'x', [], 'shared'),
  );
  assert.deepEqual(
    grapheneQueryKey(a, 'x', { b: 2, a: 1n }),
    grapheneQueryKey(a, 'x', { a: 1n, b: 2 }),
  );
  assert.notEqual(serializeCache(1n), serializeCache('1'));
  assert.notEqual(serializeCache(new Uint8Array([1])), serializeCache([1]));
  assert.notEqual(serializeCache({ a: undefined }), serializeCache({}));
  const value = {
    amount: 9007199254740993n,
    bytes: new Uint8Array([0, 255]),
    optional: undefined,
  };
  const first = cache(),
    second = cache(),
    key = grapheneQueryKey(a, 'x', [], 'ssr');
  first.setQueryData(key, value);
  hydrate(second, deserializeCache(serializeCache(dehydrate(first))));
  assert.deepEqual(second.getQueryData(key), value);
  first.clear();
  second.clear();
  assert.throws(() => serializeCache(new Date()), /plain protocol/);
  const cyclic = {};
  cyclic.self = cyclic;
  assert.throws(() => serializeCache(cyclic), /Cyclic/);
  assert.throws(() => deserializeCache('["bytes",[999]]'), /Invalid/);
});
test('generated RPC inventory covers reads and imperative methods with distinct lifecycles', async () => {
  for (const [chain, exports] of [
    ['swaplock', swaplock],
    ['bitshares', bitshares],
  ]) {
    const spec = JSON.parse(
      readFileSync(
        new URL(
          `../../rust/graphene-chain-${chain}/graphene-chain-${chain}-spec/dist/${chain}.open-graphene.json`,
          import.meta.url,
        ),
      ),
    );
    const expected = spec.rpcMethods
      .filter(
        (m) =>
          ['database', 'history', 'orders'].includes(m.apiName) &&
          /^(get_|lookup_|list_)/.test(m.name) &&
          !m.isSubscription,
      )
      .map((m) => m.apiName + '.' + m.name);
    assert.deepEqual(exports.generatedQueryMethods, expected);
    assert.equal(expected.length, chain === 'swaplock' ? 39 : 24);
    assert.equal(
      typeof exports.useNetworkBroadcastBroadcastTransaction,
      'function',
    );
    assert.equal(exports.generatedRpcMutations.length, 8);
    assert.deepEqual(
      [
        ...exports.generatedQueryMethods,
        ...exports.generatedRpcMutations,
      ].sort(),
      spec.rpcMethods.map((m) => m.apiName + '.' + m.name).sort(),
    );
  }
  const client = {
    chainId: CHAIN.chainId,
    rpc: { invoke: async () => [{ amount: 5n, asset_id: '1.3.0' }] },
  };
  const q = cache();
  const options = swaplock.databaseGetAccountBalancesOptions(client, {
    account_name_or_id: 'alice',
    assets: [],
  });
  assert.deepEqual(await q.fetchQuery(options), [
    { amount: 5n, asset_id: '1.3.0' },
  ]);
  assert.throws(
    () =>
      swaplock.databaseGetObjectsOptions(client, { ids: [], subscribe: true }),
    /live hook/,
  );
  q.clear();
});
test('live entries share one stream, survive Strict Mode re-acquire and clean up on last consumer', async () => {
  const q = cache(),
    client = {},
    key = ['test'];
  let opens = 0,
    closes = 0,
    stream;
  const open = async () => {
    opens++;
    stream = new RpcSubscription(1, () => closes++);
    return stream;
  };
  const e = sharedLive(q, client, key, open, async () => {}, {});
  assert.equal(
    e,
    sharedLive(q, client, key, open, async () => {}, {}),
  );
  const release1 = e.acquire();
  release1();
  const release2 = e.acquire(),
    release3 = e.acquire();
  await pause();
  assert.equal(opens, 1);
  stream.push({ value: 7n });
  await until(() => e.getSnapshot().status === 'live');
  assert.deepEqual(q.getQueryData(key), { value: 7n });
  release2();
  await pause();
  assert.equal(closes, 0);
  release3();
  await pause();
  assert.equal(closes, 1);
  assert.equal(e.getSnapshot().status, 'disabled');
  q.clear();
});
test('pending stream creation is closed after unmount and cannot write stale data', async () => {
  const q = cache();
  let resolve,
    closed = 0;
  const stream = new RpcSubscription(1, () => closed++);
  const e = new LiveEntry(
    q,
    ['late'],
    () => new Promise((r) => (resolve = r)),
    async () => {},
  );
  const release = e.acquire();
  release();
  await pause();
  resolve(stream);
  await pause();
  assert.equal(closed, 1);
  assert.equal(q.getQueryData(['late']), undefined);
  q.clear();
});
test('subscription snapshot cancels an older pending read', async () => {
  const q = cache(),
    key = ['race'];
  let resolve;
  const old = q
    .fetchQuery({
      queryKey: key,
      queryFn: () => new Promise((r) => (resolve = r)),
    })
    .catch(() => {});
  const stream = new RpcSubscription(1, () => {}),
    e = new LiveEntry(
      q,
      key,
      async () => stream,
      async () => {},
    ),
    release = e.acquire();
  stream.push({ value: 'new' });
  await until(() => e.getSnapshot().status === 'live');
  resolve({ value: 'old' });
  await old;
  await pause();
  assert.deepEqual(q.getQueryData(key), { value: 'new' });
  release();
  await pause();
  q.clear();
});
test('disconnect reconnects, recreates subscription and obtains fresh snapshot; permissions do not retry', async () => {
  const q = cache(),
    streams = [];
  let reconnects = 0;
  const e = new LiveEntry(
    q,
    ['recover'],
    async () => {
      const s = new RpcSubscription(1, () => {});
      streams.push(s);
      return s;
    },
    async () => {
      reconnects++;
    },
    { delayMs: 0, maxRetries: 2 },
  );
  const release = e.acquire();
  await pause();
  streams[0].push(1);
  await until(() => e.getSnapshot().status === 'live');
  streams[0].fail(new RpcTransportError('Connection closed', true));
  await until(() => streams.length === 2);
  assert.equal(reconnects, 1);
  streams[1].push(2);
  await until(() => q.getQueryData(['recover']) === 2);
  streams[1].fail(new RpcRemoteError({ message: 'Access denied' }));
  await until(() => e.getSnapshot().status === 'error');
  assert.equal(streams.length, 2);
  e.restart();
  await until(() => streams.length === 3);
  release();
  await pause();
  q.clear();
});
test('overflow resnapshots without reconnect; retry budget stops repeated empty streams', async () => {
  const q = cache(),
    streams = [];
  let reconnects = 0;
  const e = new LiveEntry(
    q,
    ['overflow'],
    async () => {
      const s = new RpcSubscription(1, () => {});
      streams.push(s);
      return s;
    },
    async () => {
      reconnects++;
    },
    { delayMs: 0, maxRetries: 1 },
  );
  const release = e.acquire();
  await pause();
  streams[0].fail(
    new Error('Subscription buffer overflow; resnapshot required'),
  );
  await until(() => streams.length === 2);
  assert.equal(reconnects, 0);
  streams[1].close();
  await until(() => e.getSnapshot().status === 'error');
  assert.equal(streams.length, 2);
  release();
  await pause();
  q.clear();
});
test('explicit stream end resubscribes; final unmount cancels scheduled reconnect', async () => {
  const q = cache(),
    streams = [];
  let reconnects = 0;
  const e = new LiveEntry(
    q,
    ['end'],
    async () => {
      const s = new RpcSubscription(1, () => {});
      streams.push(s);
      return s;
    },
    async () => {
      reconnects++;
    },
    { delayMs: 0 },
  );
  const release = e.acquire();
  await pause();
  streams[0].close();
  await until(() => streams.length === 2);
  assert.equal(reconnects, 0);
  streams[1].fail(new RpcTransportError('closed', true));
  release();
  await pause();
  await pause();
  assert.equal(reconnects, 0);
  assert.equal(e.getSnapshot().status, 'disabled');
  q.clear();
});
test('public options for explicit hydration scopes share data without mixing chain IDs', async () => {
  const q = cache();
  const a = {
      chainId: CHAIN.chainId,
      database: { account: async () => ({ name: 'alice' }) },
    },
    b = { ...a };
  const server = swaplock.accountOptions(a, 'alice', 'public-ssr');
  await q.fetchQuery(server);
  assert.equal(
    q.getQueryData(swaplock.accountOptions(b, 'alice', 'public-ssr').queryKey)
      .name,
    'alice',
  );
  assert.equal(
    q.getQueryData(swaplock.accountOptions(b, 'alice').queryKey),
    undefined,
  );
  q.clear();
});
test('transient reconnect failures consume the same bounded retry budget', async () => {
  const q = cache();
  let reconnects = 0,
    opens = 0;
  const e = new LiveEntry(
    q,
    ['reconnect-error'],
    async () => {
      opens++;
      throw new RpcTransportError('offline', false);
    },
    async () => {
      reconnects++;
      throw new RpcTransportError('still offline', false);
    },
    { delayMs: 0, maxRetries: 2 },
  );
  const release = e.acquire();
  await until(() => e.getSnapshot().status === 'error');
  assert.equal(opens, 1);
  assert.equal(reconnects, 2);
  release();
  await pause();
  q.clear();
});

const operationSymbol = (name) =>
  name === 'transfer'
    ? 'TransferOperation'
    : name
        .split('_')
        .map((p) => p[0].toUpperCase() + p.slice(1))
        .join('');
test('every generated operation preparation preserves the independently verified payload and transaction options', async () => {
  for (const [chain, api, b] of [
    ['swaplock', swaplock, swaplockBindings],
    ['bitshares', bitshares, bitsharesBindings],
  ]) {
    const spec = JSON.parse(
      readFileSync(
        new URL(
          `../../rust/graphene-chain-${chain}/graphene-chain-${chain}-spec/dist/${chain}.open-graphene.json`,
          import.meta.url,
        ),
      ),
    );
    const names = spec.operations
      .filter((op) => !op.isVirtual)
      .map((op) => op.name.replace(/_operation$/, ''));
    assert.deepEqual(api.generatedPrepareOperations, names);
    assert.equal(names.length, chain === 'swaplock' ? 88 : 71);
    const client = {
      prepareOperations: async (operations, options) => ({
        operations,
        options,
      }),
    };
    const fixture = JSON.parse(
      readFileSync(
        new URL(`fixtures/${chain}-fc-parity-rich.json`, import.meta.url),
      ),
    );
    for (const vector of fixture.vectors) {
      const name = vector.name.replace(/_operation$/, ''),
        symbol = operationSymbol(name);
      assert.equal(typeof api['usePrepare' + symbol], 'function');
      const op = b.OperationCodec.decode(vector.operation),
        options = { maxFee: 9007199254740993n, expirationSeconds: 120 };
      const result = await api['prepare' + symbol](client, op[1], options);
      assert.equal(
        bytesToHex(b.encodeOperation(result.operations[0])),
        vector.rust.hex,
        name,
      );
      assert.equal(result.options, options);
    }
    for (const op of spec.operations.filter((op) => op.isVirtual))
      assert.equal(
        api['usePrepare' + operationSymbol(op.name.replace(/_operation$/, ''))],
        undefined,
      );
  }
});
test('room preparation supplies fee/extensions defaults without generating keys or broadcasting', async () => {
  let submitted = 0;
  const client = {
    prepareOperations: async (operations, options) => ({ operations, options }),
    broadcast: () => {
      submitted++;
    },
  };
  const result = await swaplock.prepareDataRoomCreate(client, {
    owner: swaplockBindings.AccountId('1.2.100'),
    name: 'room',
    description: '',
    subject: [0, {}],
  });
  assert.deepEqual(result.operations[0], [
    78,
    {
      owner: '1.2.100',
      name: 'room',
      description: '',
      subject: [0, {}],
      fee: { amount: 0n, asset_id: '1.3.0' },
      extensions: {},
    },
  ]);
  assert.equal(submitted, 0);
  assert.ok(!('room_key' in result.operations[0][1]));
});
test('chain React packages have isolated dependency graphs and browser bundles', async () => {
  const { build } = await import('esbuild');
  const { fileURLToPath } = await import('node:url');
  const root = new URL('../', import.meta.url);
  const manifests = new Map();
  const folders = [
    'graphene-react-core',
    'graphene-transport',
    'graphene-codec',
    'graphene-fc',
    'graphene-primitives',
    'graphene-core',
  ];
  for (const chain of ['swaplock', 'bitshares'])
    for (const suffix of ['api', 'bindings', 'react'])
      folders.push(`graphene-chain-${chain}/graphene-chain-${chain}-${suffix}`);
  for (const folder of folders) {
    const manifest = JSON.parse(
      readFileSync(new URL(folder + '/package.json', root)),
    );
    manifests.set(manifest.name, manifest);
  }
  for (const chain of ['swaplock', 'bitshares']) {
    const other = chain === 'swaplock' ? 'bitshares' : 'swaplock',
      name = `@open-graphene/chain-${chain}-react`,
      visited = new Set();
    function visit(name) {
      if (visited.has(name)) return;
      visited.add(name);
      for (const dependency of Object.keys(
        manifests.get(name)?.dependencies ?? {},
      ))
        visit(dependency);
    }
    visit(name);
    assert.ok(
      ![...visited].some((name) => name.includes('chain-' + other)),
      `${chain} pulls in ${other}`,
    );
    const bundle = await build({
      stdin: {
        contents: `export * from '${name}';`,
        resolveDir: fileURLToPath(root),
        sourcefile: 'isolation.mjs',
      },
      bundle: true,
      format: 'esm',
      platform: 'browser',
      write: false,
      metafile: true,
    });
    assert.ok(
      !Object.keys(bundle.metafile.inputs).some((path) =>
        path.includes('chain-' + other),
      ),
      `${chain} browser bundle includes ${other}`,
    );
  }
});
