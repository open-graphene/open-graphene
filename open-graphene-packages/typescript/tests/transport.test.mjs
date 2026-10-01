import test from 'node:test';
import assert from 'node:assert/strict';
import {
  RpcClient,
  RpcTransportError,
  RpcRemoteError,
} from '../graphene-transport/dist/index.js';

class Socket extends EventTarget {
  readyState = 0;
  sent = [];
  held = [];
  constructor() {
    super();
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
  send(text) {
    const m = JSON.parse(text);
    this.sent.push(m);
    const method = m.params[1];
    if (method === 'login') queueMicrotask(() => this.reply(m.id, true));
    else if (method === 'database') queueMicrotask(() => this.reply(m.id, 2));
    else if (method === 'reject')
      queueMicrotask(() =>
        this.dispatchEvent(
          new MessageEvent('message', {
            data: JSON.stringify({ id: m.id, error: { message: 'rejected' } }),
          }),
        ),
      );
    else this.held.push(m);
  }
  close() {
    this.readyState = 3;
    this.dispatchEvent(new Event('close'));
  }
}
test('RPC multiplexes reordered replies, preserves wide integers and discovers API once', async () => {
  let socket;
  const client = await RpcClient.connect('ws://fixture', {
    createSocket: () => (socket = new Socket()),
  });
  try {
    const first = client.request('database', 'first', []);
    const second = client.request('database', 'second', []);
    await new Promise((resolve) => setTimeout(resolve, 0));
    const [a, b] = socket.held;
    socket.reply(b.id, 'second');
    socket.dispatchEvent(
      new MessageEvent('message', {
        data: `{"id":${a.id},"result":9007199254740993}`,
      }),
    );
    assert.equal((await first).value, '9007199254740993');
    assert.equal(await second, 'second');
    assert.equal(
      socket.sent.filter((m) => m.params[1] === 'database').length,
      1,
    );
    await assert.rejects(
      client.request('database', 'reject', []),
      RpcRemoteError,
    );
  } finally {
    client.close();
  }
});
test('sent timeout is explicit and never causes automatic retry', async () => {
  let socket;
  const client = await RpcClient.connect('ws://fixture', {
    timeoutMs: 20,
    createSocket: () => (socket = new Socket()),
  });
  try {
    await assert.rejects(
      client.request('database', 'broadcast_transaction', []),
      (e) => e instanceof RpcTransportError && e.sent,
    );
    assert.equal(
      socket.sent.filter((m) => m.params[1] === 'broadcast_transaction').length,
      1,
    );
  } finally {
    client.close();
  }
});
test('closing connection rejects every outstanding request', async () => {
  let socket;
  const client = await RpcClient.connect('ws://fixture', {
    createSocket: () => (socket = new Socket()),
  });
  const pending = client.request('database', 'pending', []);
  const rejected = assert.rejects(pending, RpcTransportError);
  await new Promise((resolve) => setTimeout(resolve, 0));
  socket.close();
  await rejected;
});
