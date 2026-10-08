import test from 'node:test';
import assert from 'node:assert/strict';
import {
  RpcClient,
  GrapheneSession,
  createSharedConnection,
  RpcSubscription,
  ChainStore,
} from '../graphene-transport/dist/index.js';
import { rpc, text } from '../graphene-codec/dist/index.js';
const chain = 'ab'.repeat(32);
class Socket extends EventTarget {
  readyState = 0;
  sent = [];
  callback = undefined;
  constructor(index = 0, handler = () => undefined) {
    super();
    this.index = index;
    this.handler = handler;
    queueMicrotask(() => {
      this.readyState = 1;
      this.dispatchEvent(new Event('open'));
    });
  }
  reply(id, result) {
    this.dispatchEvent(
      new MessageEvent('message', { data: JSON.stringify({ id, result }) }),
    );
  }
  notice(id, payload) {
    this.dispatchEvent(
      new MessageEvent('message', {
        data: JSON.stringify({
          method: 'notice',
          params: [String(id), payload],
        }),
      }),
    );
  }
  send(raw) {
    const m = JSON.parse(raw);
    this.sent.push(m);
    const method = m.params[1];
    queueMicrotask(() => {
      const override = this.handler(m, this);
      if (override === 'held') return;
      if (override !== undefined) {
        this.reply(m.id, override);
        return;
      }
      if (method === 'login') this.reply(m.id, true);
      else if (method === 'database') this.reply(m.id, 2 + this.index);
      else if (method === 'network_broadcast') this.reply(m.id, 3 + this.index);
      else if (method === 'get_chain_id') this.reply(m.id, chain);
      else if (method === 'set_subscribe_callback') {
        this.callback = m.params[2][0];
        this.reply(m.id, null);
      } else if (method === 'get_objects')
        this.reply(m.id, [{ id: '1.2.1', name: 'initial' }]);
      else if (method === 'get_probe')
        this.reply(m.id, 'connected-' + this.index);
      else if (method === 'subscribe_to_market') {
        this.notice(m.params[2][0], [{ early: true }]);
        this.reply(m.id, null);
      } else throw Error('Unexpected ' + method);
    });
  }
  close() {
    if (this.readyState === 3) return;
    this.readyState = 3;
    this.dispatchEvent(new Event('close'));
  }
}
const probe = rpc('database', 'get_probe', [], text);
test('session retries reads on a fresh connection and rediscovers API IDs', async () => {
  let attempts = 0;
  const sockets = [];
  const session = await GrapheneSession.connect('ws://fixture', {
    expectedChainId: chain,
    reconnect: { maxRetries: 1, delayMs: 0 },
    createSocket: () => {
      const index = attempts++;
      const socket = new Socket(index, (m, s) => {
        if (index === 0 && m.params[1] === 'get_probe') {
          s.close();
          return 'held';
        }
      });
      sockets.push(socket);
      return socket;
    },
  });
  try {
    assert.equal(await session.invoke(probe, {}), 'connected-1');
    assert.equal(attempts, 2);
    assert.equal(
      sockets[1].sent.find((m) => m.params[1] === 'get_probe').params[0],
      3,
    );
  } finally {
    session.close();
  }
});
test('session fails over endpoints but rejects a wrong chain; close prevents reconnect', async () => {
  let socket;
  const session = await GrapheneSession.connect(['ws://wrong', 'ws://right'], {
    expectedChainId: chain,
    createSocket: (url) =>
      (socket = new Socket(0, (m) =>
        m.params[1] === 'get_chain_id' && url.endsWith('wrong')
          ? 'ff'.repeat(32)
          : undefined,
      )),
  });
  assert.equal(session.endpoint, 'ws://right');
  session.close();
  await assert.rejects(session.reconnect(), /closed/);
  assert.equal(socket.readyState, 3);
});
test('callbacks arriving before registration response are retained and string IDs route correctly', async () => {
  const client = await RpcClient.connect('ws://fixture', {
    createSocket: () => new Socket(),
  });
  try {
    const stream = await client.subscribe(
      'database',
      'subscribe_to_market',
      (id) => [id, '1.3.0', '1.3.1'],
    );
    assert.equal((await stream.nextTimeout(100)).value[0].early, true);
    stream.close();
    assert.equal((await stream.next()).done, true);
  } finally {
    client.close();
  }
});
test('timeout does not consume the next callback; overflow fails instead of dropping events', async () => {
  const stream = new RpcSubscription(1, () => {}, 2);
  await assert.rejects(stream.nextTimeout(5), /timeout/);
  stream.push('later');
  assert.equal((await stream.next()).value, 'later');
  stream.push(1);
  stream.push(2);
  stream.push(3);
  await assert.rejects(stream.next(), /overflow/);
});
test('database callback fans out independently; ChainStore applies ID-only deletes and isolates snapshots', async () => {
  let socket;
  const session = await GrapheneSession.connect('ws://fixture', {
    expectedChainId: chain,
    createSocket: () => (socket = new Socket()),
  });
  try {
    const store = await ChainStore.create(session, ['1.2.1']),
      other = await session.databaseNotices();
    assert.equal(
      socket.sent.filter((m) => m.params[1] === 'set_subscribe_callback')
        .length,
      1,
    );
    const copy = store.get('1.2.1');
    copy.name = 'tampered';
    assert.equal(store.get('1.2.1').name, 'initial');
    socket.notice(socket.callback, [[{ id: '1.2.1', name: 'updated' }]]);
    await store.changes.nextTimeout(100);
    assert.equal(store.get('1.2.1').name, 'updated');
    socket.notice(socket.callback, [['1.2.1']]);
    await store.changes.nextTimeout(100);
    assert.equal(store.get('1.2.1'), null);
    store.close();
    assert.equal((await other.nextTimeout(100)).done, false);
    other.close();
  } finally {
    session.close();
  }
});
test('reconnect terminates old subscriptions explicitly instead of silently retaining stale caches', async () => {
  const session = await GrapheneSession.connect('ws://fixture', {
    expectedChainId: chain,
    createSocket: () => new Socket(),
  });
  const notices = await session.databaseNotices();
  const pending = assert.rejects(notices.next(), /closed/);
  await session.reconnect();
  await pending;
  assert.equal(await session.invoke(probe, {}), 'connected-0');
  session.close();
});

test('shared connection coalesces consumers and leases close independently', async () => {
  const sockets = [];
  const connection = createSharedConnection({
    endpoints: ['ws://fixture'],
    expectedChainId: chain,
    createSocket() {
      const socket = new Socket();
      sockets.push(socket);
      return socket;
    },
  });
  const [first, second] = await Promise.all([
    GrapheneSession.connect('ws://fixture', {
      connection,
      expectedChainId: chain,
    }),
    GrapheneSession.connect('ws://fixture', {
      connection,
      expectedChainId: chain,
    }),
  ]);
  try {
    assert.equal(sockets.length, 1);
    const notices = await first.databaseNotices();
    first.close();
    assert.equal((await notices.next()).done, true);
    assert.equal(sockets[0].readyState, 1);
    assert.equal(await second.invoke(probe, {}), 'connected-0');
    connection.close();
    assert.equal(sockets[0].readyState, 3);
    await assert.rejects(second.invoke(probe, {}));
  } finally {
    first.close();
    second.close();
    connection.close();
  }
});

test('shared request cancellation leaves other callers and the socket active', async () => {
  let held;
  let socket;
  const connection = createSharedConnection({
    endpoints: ['ws://fixture'],
    expectedChainId: chain,
    createSocket() {
      socket = new Socket(0, (message) => {
        if (message.params[1] === 'get_held') {
          held = message.id;
          return 'held';
        }
      });
      return socket;
    },
  });
  const cancel = new AbortController();
  const first = await GrapheneSession.connect('ws://fixture', {
    connection,
    signal: cancel.signal,
  });
  const second = await GrapheneSession.connect('ws://fixture', { connection });
  try {
    const read = first.request('database', 'get_held', []);
    const rejected = assert.rejects(read, /cancel query/);
    await new Promise((resolve) => setTimeout(resolve, 0));
    cancel.abort(new Error('cancel query'));
    await rejected;
    assert.equal(await second.invoke(probe, {}), 'connected-0');
    socket.reply(held, 'late');
    assert.equal(socket.readyState, 1);
  } finally {
    first.close();
    second.close();
    connection.close();
  }
});

test('shared reconnect is coalesced, retires the old socket, and keeps chain pinning', async () => {
  const sockets = [];
  const connection = createSharedConnection({
    endpoints: ['ws://fixture'],
    expectedChainId: chain,
    createSocket() {
      assert.equal(
        sockets.filter((socket) => socket.readyState === 1).length,
        0,
      );
      const socket = new Socket(sockets.length);
      sockets.push(socket);
      return socket;
    },
  });
  const first = await GrapheneSession.connect('ws://fixture', { connection });
  const second = await GrapheneSession.connect('ws://fixture', { connection });
  try {
    await Promise.all([first.reconnect(), second.reconnect()]);
    assert.equal(sockets.length, 2);
    assert.equal(await first.invoke(probe, {}), 'connected-1');
    assert.equal(await second.invoke(probe, {}), 'connected-1');
    await assert.rejects(
      GrapheneSession.connect('ws://fixture', {
        connection,
        expectedChainId: 'cd'.repeat(32),
      }),
      /different chain/,
    );
    assert.equal(sockets.length, 2);
  } finally {
    first.close();
    second.close();
    connection.close();
  }
});

test('shared connection never retries a submitted mutation after transport failure', async () => {
  let submitted = 0;
  let opened = 0;
  const connection = createSharedConnection({
    endpoints: ['ws://fixture'],
    expectedChainId: chain,
    createSocket() {
      opened += 1;
      return new Socket(0, (message, socket) => {
        if (message.params[1] === 'broadcast_transaction') {
          submitted += 1;
          socket.close();
          return 'held';
        }
      });
    },
  });
  const session = await GrapheneSession.connect('ws://fixture', { connection });
  try {
    await assert.rejects(
      session.request('network_broadcast', 'broadcast_transaction', []),
    );
    assert.equal(submitted, 1);
    assert.equal(opened, 1);
  } finally {
    session.close();
    connection.close();
  }
});
