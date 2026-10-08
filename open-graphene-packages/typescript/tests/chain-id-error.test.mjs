import test from 'node:test';
import assert from 'node:assert/strict';
import { getEventListeners } from 'node:events';
import {
  GrapheneSession,
  ChainIdMismatchError,
  hasChainIdMismatch,
} from '../graphene-transport/dist/index.js';
import {
  SwaplockClient,
  hasChainIdMismatch as clientHasChainIdMismatch,
} from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';

const expectedChainId = 'ab'.repeat(32);
const otherChainId = 'cd'.repeat(32);

class Socket extends EventTarget {
  readyState = 0;
  closeCalls = 0;
  methods = [];

  constructor(options) {
    super();
    this.options = options;
    queueMicrotask(() => {
      if (options.unavailable) {
        this.dispatchEvent(new Event('error'));
        return;
      }

      this.readyState = 1;
      this.dispatchEvent(new Event('open'));
    });
  }

  send(raw) {
    const request = JSON.parse(raw);
    const method = request.params[1];
    this.methods.push(method);

    let result = 2;
    if (method === 'login') {
      result = true;
    } else if (method === 'get_chain_id') {
      result = this.options.chainId;
      this.options.onChainId?.();
    }

    const data = JSON.stringify({
      id: request.id,
      result,
    });
    queueMicrotask(() => {
      const response = new MessageEvent('message', {
        data,
      });

      this.dispatchEvent(response);
    });
  }

  close() {
    this.closeCalls += 1;
    this.readyState = 3;
    this.dispatchEvent(new Event('close'));
  }
}

function connections(servers) {
  const sockets = [];
  const opened = [];

  return {
    sockets,
    opened,
    createSocket(endpoint) {
      opened.push(endpoint);
      const socket = new Socket(servers[endpoint]);
      sockets.push(socket);

      return socket;
    },
  };
}

function mismatch() {
  return new ChainIdMismatchError({
    expectedChainId,
    actualChainId: otherChainId,
  });
}

test('one incompatible endpoint reports both chain IDs and releases the socket', async () => {
  const state = connections({
    'ws://wrong': {
      chainId: otherChainId,
    },
  });
  await assert.rejects(
    GrapheneSession.connect('ws://wrong', {
      expectedChainId,
      createSocket: state.createSocket,
    }),
    (error) => {
      assert.ok(error instanceof ChainIdMismatchError);
      assert.equal(error.expectedChainId, expectedChainId);
      assert.equal(error.actualChainId, otherChainId);
      assert.equal(hasChainIdMismatch(error), true);
      assert.equal(clientHasChainIdMismatch(error), true);
      return true;
    },
  );
  assert.equal(state.sockets[0].closeCalls, 1);
  assert.equal(getEventListeners(state.sockets[0], 'message').length, 0);
});

for (const strategy of ['first-available', 'lowest-latency']) {
  test(`${strategy} selects the compatible endpoint despite mismatch and connection failure`, async () => {
    const state = connections({
      'ws://wrong': {
        chainId: otherChainId,
      },
      'ws://offline': {
        unavailable: true,
      },
      'ws://right': {
        chainId: expectedChainId,
      },
    });
    const client = await SwaplockClient.connect(
      ['ws://wrong', 'ws://offline', 'ws://right'],
      {
        expectedChainId,
        strategy,
        createSocket: state.createSocket,
      },
    );
    try {
      assert.equal(client.chainId, expectedChainId);
      assert.equal(client.rpc.endpoint, 'ws://right');
      const incompatible = state.sockets.filter(
        (socket) => socket.options.chainId === otherChainId,
      );
      assert.ok(incompatible.every((socket) => socket.closeCalls === 1));
      assert.ok(
        incompatible.every((socket) =>
          socket.methods.every((method) =>
            ['login', 'database', 'get_chain_id'].includes(method),
          ),
        ),
      );
    } finally {
      client.close();
    }
  });
}

for (const scenario of ['all-wrong', 'mixed', 'offline']) {
  test(`aggregate classification for ${scenario} endpoints keeps any mismatch distinct from unavailability`, async () => {
    const first =
      scenario === 'offline'
        ? {
            unavailable: true,
          }
        : {
            chainId: otherChainId,
          };
    const second =
      scenario === 'all-wrong'
        ? {
            chainId: 'ef'.repeat(32),
          }
        : {
            unavailable: true,
          };
    const state = connections({
      'ws://first': first,
      'ws://second': second,
    });
    await assert.rejects(
      GrapheneSession.connect(['ws://first', 'ws://second'], {
        expectedChainId,
        createSocket: state.createSocket,
      }),
      (error) => {
        assert.ok(error instanceof AggregateError);
        assert.equal(error.errors.length, 2);
        assert.equal(hasChainIdMismatch(error), scenario !== 'offline');
        return true;
      },
    );
  });
}

test('failed latency probes retain typed chain mismatches', async () => {
  const state = connections({
    'ws://wrong': {
      chainId: otherChainId,
    },
  });
  await assert.rejects(
    GrapheneSession.probeLatencies(['ws://wrong'], {
      expectedChainId,
      createSocket: state.createSocket,
    }),
    (error) => error instanceof AggregateError && hasChainIdMismatch(error),
  );
});

for (const chainId of [null, 'invalid', 'AB'.repeat(32)]) {
  test(`malformed chain ID ${chainId} is not a valid network mismatch`, async () => {
    const state = connections({
      'ws://malformed': {
        chainId,
      },
    });
    await assert.rejects(
      GrapheneSession.connect('ws://malformed', {
        expectedChainId,
        createSocket: state.createSocket,
      }),
      (error) =>
        error.message === 'Invalid chain ID' && !hasChainIdMismatch(error),
    );
  });
}

test('reconnection rejects a node that changes its chain ID', async () => {
  let changed = false;
  const sockets = [];
  const session = await GrapheneSession.connect('ws://fixture', {
    expectedChainId,
    createSocket() {
      const socket = new Socket({
        chainId: changed ? otherChainId : expectedChainId,
      });
      sockets.push(socket);
      return socket;
    },
  });
  try {
    changed = true;
    await assert.rejects(session.reconnect(), ChainIdMismatchError);
    assert.equal(session.chainId, expectedChainId);
    assert.equal(sockets[1].closeCalls, 1);
  } finally {
    session.close();
  }
});

test('cancellation takes precedence over mismatch and prevents fallback', async () => {
  const controller = new AbortController();
  const reason = new Error('selection ended');
  const state = connections({
    'ws://wrong': {
      chainId: otherChainId,
      onChainId: () => controller.abort(reason),
    },
    'ws://right': {
      chainId: expectedChainId,
    },
  });
  await assert.rejects(
    GrapheneSession.connect(['ws://wrong', 'ws://right'], {
      expectedChainId,
      signal: controller.signal,
      createSocket: state.createSocket,
    }),
    (error) => error === reason && !hasChainIdMismatch(error),
  );
  assert.deepEqual(state.opened, ['ws://wrong']);
});

test('message lookalikes and unrelated errors are not chain mismatches', () => {
  for (const value of [
    null,
    undefined,
    'failure',
    new Error('Connected node has a different chain ID'),
    {
      name: 'ChainIdMismatchError',
    },
    new AggregateError([]),
  ]) {
    assert.equal(hasChainIdMismatch(value), false);
  }
});

test('nested and cyclic aggregate errors are inspected without recursion', () => {
  const loop = new AggregateError([]);
  loop.errors.push(loop);
  assert.equal(hasChainIdMismatch(loop), false);

  let nested = mismatch();
  for (let depth = 0; depth < 10000; depth += 1) {
    nested = new AggregateError([nested]);
  }

  loop.errors.push(nested);
  assert.equal(hasChainIdMismatch(loop), true);
});
