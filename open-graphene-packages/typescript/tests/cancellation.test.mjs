import test from 'node:test';
import assert from 'node:assert/strict';
import { getEventListeners } from 'node:events';
import { setImmediate as nextTurn } from 'node:timers/promises';
import {
  RpcClient,
  GrapheneSession,
} from '../graphene-transport/dist/index.js';
import { SwaplockClient } from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import { CHAIN } from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { rpc, text } from '../graphene-codec/dist/index.js';

// Closing deliberately has no acknowledgement: cancellation must settle locally.
class Socket extends EventTarget {
  readyState = 0;
  closeCalls = 0;
  requests = [];
  heldMethod;

  constructor(options = {}) {
    super();
    this.heldMethod = options.heldMethod;

    if (options.open !== false) {
      queueMicrotask(() => this.open());
    }
  }

  open() {
    this.readyState = 1;
    this.dispatchEvent(new Event('open'));
  }

  send(raw) {
    const request = JSON.parse(raw);
    this.requests.push(request);
    this.dispatchEvent(new Event('request'));
    const method = request.params[1];

    if (method === this.heldMethod) {
      return;
    }

    let result = null;

    switch (method) {
      case 'login':
        result = true;
        break;
      case 'database':
      case 'network_broadcast':
        result = 2;
        break;
      case 'get_chain_id':
        result = CHAIN.chainId;
        break;
      case 'get_probe':
        result = 'ready';
        break;
    }

    queueMicrotask(() => this.reply(request.id, result));
  }

  reply(id, result) {
    const envelope = JSON.stringify({
      id,
      result,
    });
    this.dispatchEvent(
      new MessageEvent('message', {
        data: envelope,
      }),
    );
  }

  waitForRequest(method) {
    return new Promise((resolve) => {
      const find = () => {
        const request = this.requests.find((item) => item.params[1] === method);

        if (request) {
          this.removeEventListener('request', find);
          resolve(request);
        }
      };

      this.addEventListener('request', find);
      find();
    });
  }

  close() {
    this.closeCalls += 1;
    this.readyState = 2;
  }

  disconnect() {
    this.readyState = 3;
    this.dispatchEvent(new Event('close'));
  }
}

function assertReleased(socket, signal) {
  for (const event of ['open', 'message', 'close', 'error']) {
    assert.equal(getEventListeners(socket, event).length, 0, event);
  }

  assert.equal(getEventListeners(signal, 'abort').length, 0);
}

const probe = rpc('database', 'get_probe', [], text);

for (const Client of [RpcClient, GrapheneSession, SwaplockClient]) {
  test(`${Client.name} rejects a pre-aborted signal without opening a socket`, async () => {
    const controller = new AbortController();
    const reason = new Error('selection ended');
    controller.abort(reason);
    let opened = 0;

    await assert.rejects(
      Client.connect('ws://fixture', {
        expectedChainId: CHAIN.chainId,
        signal: controller.signal,
        createSocket() {
          opened += 1;
          return new Socket();
        },
      }),
      (error) => error === reason,
    );

    assert.equal(opened, 0);
  });
}

test('cancellation closes a socket even when its factory cancels before returning', async () => {
  const controller = new AbortController();
  const socket = new Socket({
    open: false,
  });

  await assert.rejects(
    RpcClient.connect('ws://fixture', {
      signal: controller.signal,
      createSocket() {
        controller.abort();
        return socket;
      },
    }),
    (error) => error === controller.signal.reason,
  );

  assert.equal(socket.closeCalls, 1);
  assertReleased(socket, controller.signal);
});

for (const stage of ['open', 'login', 'database', 'get_chain_id']) {
  test(
    `cancellation interrupts ${stage} without close acknowledgement or endpoint fallback`,
    {
      timeout: 1000,
    },
    async (t) => {
      const controller = new AbortController();
      t.after(() => controller.abort());
      const reason = new Error('cancel setup');
      const socket = new Socket({
        open: stage !== 'open',
        heldMethod: stage,
      });
      const endpoints = [];
      const connection = GrapheneSession.connect(
        ['ws://first', 'ws://second'],
        {
          signal: controller.signal,
          timeoutMs: 60000,
          createSocket(endpoint) {
            endpoints.push(endpoint);
            return socket;
          },
        },
      );
      const rejected = assert.rejects(connection, (error) => error === reason);

      if (stage !== 'open') {
        await socket.waitForRequest(stage);
      }

      controller.abort(reason);
      await rejected;
      socket.open();
      socket.reply(1, true);
      await nextTurn();

      assert.deepEqual(endpoints, ['ws://first']);
      assert.equal(socket.closeCalls, 1);
      assertReleased(socket, controller.signal);
      assert.equal(
        socket.requests.some((item) => item.params[1] === 'get_probe'),
        false,
      );
    },
  );
}

test(
  'cancellation rejects all reads and subscriptions, including falsy reasons and late replies',
  {
    timeout: 1000,
  },
  async (t) => {
    const controller = new AbortController();
    const socket = new Socket({
      heldMethod: 'get_probe',
    });
    const client = await RpcClient.connect('ws://fixture', {
      signal: controller.signal,
      createSocket: () => socket,
    });
    t.after(() => client.close());
    const stream = await client.subscribe('database', 'subscribe', (id) => [
      id,
    ]);
    const firstRead = client.request('database', 'get_probe', []);
    const secondRead = client.request('database', 'get_probe', []);
    const failed = Promise.all([
      assert.rejects(firstRead, (reason) => reason === null),
      assert.rejects(secondRead, (reason) => reason === null),
      assert.rejects(stream.nextTimeout(60000), (reason) => reason === null),
    ]);
    const request = await socket.waitForRequest('get_probe');

    controller.abort(null);
    socket.reply(request.id, 'late result');
    await failed;
    await assert.rejects(stream.next(), (reason) => reason === null);
    const sentBefore = socket.requests.length;
    await assert.rejects(
      client.request('database', 'get_probe', []),
      (reason) => reason === null,
    );
    client.close();

    assert.equal(socket.requests.length, sentBefore);
    assert.equal(socket.closeCalls, 1);
    assertReleased(socket, controller.signal);
  },
);

test(
  'SwaplockClient forwards lifetime cancellation and never retries a sent broadcast',
  {
    timeout: 1000,
  },
  async (t) => {
    const controller = new AbortController();
    const socket = new Socket({
      heldMethod: 'broadcast_transaction_with_callback',
    });
    let connections = 0;
    const client = await SwaplockClient.connect('ws://fixture', {
      expectedChainId: CHAIN.chainId,
      signal: controller.signal,
      createSocket() {
        connections += 1;
        return socket;
      },
    });
    t.after(() => client.close());
    const registration = client.rpc.subscribe(
      'network_broadcast',
      'broadcast_transaction_with_callback',
      (id) => [id, {}],
    );
    const rejected = assert.rejects(
      registration,
      (error) => error === controller.signal.reason,
    );
    await socket.waitForRequest('broadcast_transaction_with_callback');

    controller.abort();
    await rejected;
    await assert.rejects(
      client.reconnect(),
      (error) => error === controller.signal.reason,
    );

    assert.equal(connections, 1);
    assert.equal(
      socket.requests.filter(
        (item) => item.params[1] === 'broadcast_transaction_with_callback',
      ).length,
      1,
    );
    assertReleased(socket, controller.signal);
  },
);

for (const ending of ['abort', 'close']) {
  test(
    `${ending} interrupts the reconnect delay`,
    {
      timeout: 1000,
    },
    async (t) => {
      const controller = new AbortController();
      const socket = new Socket({
        heldMethod: 'get_probe',
      });
      let connections = 0;
      const session = await GrapheneSession.connect('ws://fixture', {
        signal: controller.signal,
        reconnect: {
          maxRetries: 1,
          delayMs: 60000,
        },
        createSocket() {
          connections += 1;
          return socket;
        },
      });
      t.after(() => session.close());
      const reading = session.invoke(probe, {});
      const rejected = assert.rejects(
        reading,
        ending === 'abort'
          ? (error) => error === controller.signal.reason
          : /Session closed/,
      );
      await socket.waitForRequest('get_probe');
      socket.disconnect();
      await nextTurn();

      if (ending === 'abort') {
        controller.abort();
      } else {
        session.close();
      }

      await rejected;
      assert.equal(connections, 1);
      assertReleased(socket, controller.signal);
    },
  );
}

for (const ending of ['abort', 'close']) {
  test(
    `${ending} cancels a reconnect already waiting for login`,
    {
      timeout: 1000,
    },
    async (t) => {
      const controller = new AbortController();
      const sockets = [];
      const session = await GrapheneSession.connect('ws://fixture', {
        signal: controller.signal,
        createSocket() {
          const socket = new Socket({
            heldMethod: sockets.length ? 'login' : undefined,
          });
          sockets.push(socket);
          return socket;
        },
      });
      t.after(() => session.close());
      const reconnecting = session.reconnect();
      const rejected = assert.rejects(
        reconnecting,
        ending === 'abort'
          ? (error) => error === controller.signal.reason
          : /Session closed/,
      );
      await sockets[1].waitForRequest('login');

      if (ending === 'abort') {
        controller.abort();
      } else {
        session.close();
      }

      await rejected;
      assert.equal(sockets.length, 2);
      assert.equal(sockets[1].closeCalls, 1);
      assertReleased(sockets[0], controller.signal);
      assertReleased(sockets[1], controller.signal);
    },
  );
}

test(
  'cancellation ends all concurrent latency probes with the original reason',
  {
    timeout: 1000,
  },
  async (t) => {
    const controller = new AbortController();
    t.after(() => controller.abort());
    const sockets = [];
    const probing = GrapheneSession.connect(['ws://first', 'ws://second'], {
      signal: controller.signal,
      strategy: 'lowest-latency',
      createSocket() {
        const socket = new Socket({
          heldMethod: 'login',
        });
        sockets.push(socket);
        return socket;
      },
    });
    const rejected = assert.rejects(
      probing,
      (error) => error === controller.signal.reason,
    );
    await Promise.all(sockets.map((socket) => socket.waitForRequest('login')));

    controller.abort();
    await rejected;

    assert.equal(sockets.length, 2);

    for (const socket of sockets) {
      assert.equal(socket.closeCalls, 1);
      assertReleased(socket, controller.signal);
    }
  },
);

for (const ending of ['close', 'disconnect', 'error', 'malformed']) {
  test(`${ending} releases socket and abort listeners`, async () => {
    const controller = new AbortController();
    const socket = new Socket();
    const client = await RpcClient.connect('ws://fixture', {
      signal: controller.signal,
      createSocket: () => socket,
    });

    if (ending === 'close') {
      client.close();
    } else if (ending === 'disconnect') {
      socket.disconnect();
    } else if (ending === 'error') {
      socket.dispatchEvent(new Event('error'));
    } else {
      socket.dispatchEvent(
        new MessageEvent('message', {
          data: '{invalid json',
        }),
      );
    }

    assertReleased(socket, controller.signal);
    const closeCalls = socket.closeCalls;
    controller.abort();
    client.close();
    assert.equal(socket.closeCalls, closeCalls);
  });
}

for (const stage of ['open', 'login']) {
  test(
    `${stage} timeout releases connection resources`,
    {
      timeout: 1000,
    },
    async () => {
      const controller = new AbortController();
      const socket = new Socket({
        open: stage !== 'open',
        heldMethod: stage,
      });

      await assert.rejects(
        RpcClient.connect('ws://fixture', {
          signal: controller.signal,
          timeoutMs: 10,
          createSocket: () => socket,
        }),
        /timeout/,
      );

      assert.equal(socket.closeCalls, 1);
      assertReleased(socket, controller.signal);
    },
  );
}

test('a refused login releases connection resources', async () => {
  const controller = new AbortController();
  const socket = new Socket({
    heldMethod: 'login',
  });
  const connection = RpcClient.connect('ws://fixture', {
    signal: controller.signal,
    createSocket: () => socket,
  });
  const rejected = assert.rejects(connection, /login refused/);
  const request = await socket.waitForRequest('login');

  socket.reply(request.id, false);
  await rejected;

  assert.equal(socket.closeCalls, 1);
  assertReleased(socket, controller.signal);
});

test('a broadcast requested after closing is reported as unsent', async () => {
  const socket = new Socket();
  const client = await RpcClient.connect('ws://fixture', {
    createSocket: () => socket,
  });
  client.close();
  const sentBefore = socket.requests.length;

  await assert.rejects(
    client.request('network_broadcast', 'broadcast_transaction', []),
    (error) => error.name === 'RpcTransportError' && error.sent === false,
  );

  assert.equal(socket.requests.length, sentBefore);
});

test('lifetime cancellation still applies after a successful reconnect', async (t) => {
  const controller = new AbortController();
  const sockets = [];
  const session = await GrapheneSession.connect('ws://fixture', {
    signal: controller.signal,
    createSocket() {
      const socket = new Socket();
      sockets.push(socket);
      return socket;
    },
  });
  t.after(() => session.close());
  await session.reconnect();
  assert.equal(await session.invoke(probe, {}), 'ready');

  controller.abort();
  await assert.rejects(
    session.invoke(probe, {}),
    (error) => error === controller.signal.reason,
  );
  await assert.rejects(
    session.reconnect(),
    (error) => error === controller.signal.reason,
  );

  assert.equal(sockets.length, 2);

  for (const socket of sockets) {
    assert.equal(socket.closeCalls, 1);
    assertReleased(socket, controller.signal);
  }
});
