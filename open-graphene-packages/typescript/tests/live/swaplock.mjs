// Test-only socket adapter; production transport/session is a later SDK stage.
import * as chain from '../../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import {
  parseJson,
  stringifyJson,
  smallInteger,
} from '../../graphene-codec/dist/index.js';

function expect(condition, message) {
  if (!condition) throw new Error(message);
}

export async function runSwaplockSmoke(endpoint) {
  const report = { endpoint, startedAt: new Date().toISOString(), checks: [] };
  const socket = new WebSocket(endpoint);
  const pending = new Map();
  let nextId = 0;
  const rejectPending = (error) => {
    for (const request of pending.values()) {
      clearTimeout(request.timer);
      request.reject(error);
    }
    pending.clear();
  };
  socket.addEventListener('message', (event) => {
    try {
      const message = parseJson(event.data);
      if (message.id === undefined) return;
      const id = smallInteger(0, Number.MAX_SAFE_INTEGER).decode(message.id);
      const request = pending.get(id);
      if (!request) return;
      pending.delete(id);
      clearTimeout(request.timer);
      if (message.error)
        request.reject(new Error(`RPC: ${stringifyJson(message.error)}`));
      else request.resolve(message.result);
    } catch (error) {
      rejectPending(error);
    }
  });
  socket.addEventListener('close', () =>
    rejectPending(new Error('WebSocket closed')),
  );
  socket.addEventListener('error', () =>
    rejectPending(new Error('WebSocket error')),
  );
  const call = (api, method, args) =>
    new Promise((resolve, reject) => {
      const id = ++nextId;
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`Timeout: ${method}`));
      }, 12000);
      pending.set(id, { resolve, reject, timer });
      try {
        socket.send(
          stringifyJson({ id, method: 'call', params: [api, method, args] }),
        );
      } catch (error) {
        clearTimeout(timer);
        pending.delete(id);
        reject(error);
      }
    });
  const check = async (name, action) => {
    const start = performance.now();
    try {
      const result = await action();
      report.checks.push({
        name,
        status: 'passed',
        milliseconds: Math.round(performance.now() - start),
      });
      return result;
    } catch (error) {
      report.checks.push({ name, status: 'failed', error: error.message });
      return undefined;
    }
  };
  try {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error('WebSocket connect timeout')),
        12000,
      );
      socket.addEventListener(
        'open',
        () => {
          clearTimeout(timer);
          resolve();
        },
        { once: true },
      );
      socket.addEventListener(
        'error',
        () => {
          clearTimeout(timer);
          reject(new Error('WebSocket connect failed'));
        },
        { once: true },
      );
    });
    expect(
      (await call(1, 'login', ['', ''])) === true,
      'Anonymous login failed',
    );
    const database = smallInteger(0, 0xffffffff).decode(
      await call(1, 'database', []),
    );
    const invoke = async (descriptor, params = {}) =>
      descriptor.parseReturns(
        await call(
          database,
          descriptor.method,
          descriptor.encodeParams(params),
        ),
      );
    report.chainId = await check(
      'chain identity matches generated spec',
      async () => {
        const id = await invoke(chain.DatabaseGetChainId);
        expect(
          id === chain.CHAIN.chainId,
          `Chain mismatch: received ${id}, expected ${chain.CHAIN.chainId}`,
        );
        return id;
      },
    );
    if (!report.chainId) return report;
    const head = await check('dynamic global properties', async () => {
      const value = await invoke(chain.DatabaseGetDynamicGlobalProperties);
      expect(value.head_block_number > 0, 'No head block');
      return value;
    });
    if (head) {
      report.headBlock = head.head_block_number;
      report.headTime = head.time;
    }
    await check('chain properties', () =>
      invoke(chain.DatabaseGetChainProperties),
    );
    await check('global properties and fee schedule variants', () =>
      invoke(chain.DatabaseGetGlobalProperties),
    );
    await check('configuration', () => invoke(chain.DatabaseGetConfig));
    await check('account lookup', async () => {
      const accounts = await invoke(chain.DatabaseLookupAccounts, {
        lower_bound_name: '',
        limit: 3,
        subscribe: false,
      });
      expect(accounts.length > 0, 'No accounts returned');
    });
    await check('accounts with missing-account null slot', async () => {
      const accounts = await invoke(chain.DatabaseGetAccounts, {
        account_names_or_ids: ['1.2.0', '1.2.281474976710655'],
        subscribe: false,
      });
      expect(
        accounts.length === 2 &&
          accounts[0]?.id === '1.2.0' &&
          accounts[1] === null,
        'Account slot mismatch',
      );
    });
    await check('full account', async () => {
      const accounts = await invoke(chain.DatabaseGetFullAccounts, {
        names_or_ids: ['1.2.0'],
        subscribe: false,
      });
      expect(accounts.length === 1, 'Full account missing');
    });
    await check('asset with missing-asset null slot', async () => {
      const assets = await invoke(chain.DatabaseGetAssets, {
        asset_symbols_or_ids: ['1.3.0', '1.3.281474976710655'],
        subscribe: false,
      });
      expect(
        assets.length === 2 && assets[0]?.id === '1.3.0' && assets[1] === null,
        'Asset slot mismatch',
      );
    });
    await check('asset listing', async () => {
      const assets = await invoke(chain.DatabaseListAssets, {
        lower_bound_symbol: '',
        limit: 3,
      });
      expect(assets.length > 0, 'No assets returned');
    });
    await check('account balances', () =>
      invoke(chain.DatabaseGetAccountBalances, {
        account_name_or_id: '1.2.0',
        assets: ['1.3.0'],
      }),
    );
    await check('heterogeneous object routing and null slot', async () => {
      const objects = await invoke(chain.DatabaseGetObjects, {
        ids: ['1.2.0', '1.3.0', '2.1.0', '1.2.281474976710655'],
        subscribe: false,
      });
      expect(
        objects.length === 4 && objects[3] === null,
        'Object slot mismatch',
      );
      expect(
        objects.slice(0, 3).every((value) => value && value.kind !== 'unknown'),
        'Expected known objects',
      );
    });
    if (head) {
      await check('head block header', async () =>
        expect(
          (await invoke(chain.DatabaseGetBlockHeader, {
            block_num: head.head_block_number,
          })) !== null,
          'Missing header',
        ),
      );
      await check('head block', async () =>
        expect(
          (await invoke(chain.DatabaseGetBlock, {
            block_num: head.head_block_number,
          })) !== null,
          'Missing block',
        ),
      );
    }
    await check('transfer fee estimation (no broadcast)', async () => {
      const transfer = chain.operation.transfer(
        chain.TransferOperationCodec.decode({
          fee: { amount: '0', asset_id: '1.3.0' },
          from: '1.2.0',
          to: '1.2.1',
          amount: { amount: '1', asset_id: '1.3.0' },
          extensions: [],
        }),
      );
      const fees = await invoke(chain.DatabaseGetRequiredFees, {
        ops: [transfer],
        asset_symbol_or_id: '1.3.0',
      });
      expect(
        fees.length === 1 && typeof fees[0].amount === 'bigint',
        'Invalid decoded fee',
      );
      report.transferFee = fees[0].amount.toString();
    });
    await check('Data Room lookup', async () => {
      const room = await invoke(chain.DatabaseGetDataRoomById, {
        room_id: '1.23.0',
      });
      report.roomFound = room !== null;
    });
    await check('Data Room missing ID returns null', async () =>
      expect(
        (await invoke(chain.DatabaseGetDataRoomById, {
          room_id: '1.23.281474976710655',
        })) === null,
        'Expected null',
      ),
    );
    await check('Data Room access state', () =>
      invoke(chain.DatabaseGetDataRoomAccessState, { room_id: '1.23.0' }),
    );
    await check('Data Room members', () =>
      invoke(chain.DatabaseGetDataRoomMembers, { room_id: '1.23.0', limit: 3 }),
    );
    await check('Content Card lookup', async () => {
      const card = await invoke(chain.DatabaseGetContentCardById, {
        content_id: '1.26.0',
      });
      report.contentCardFound = card !== null;
    });
    await check('Content Cards by room', () =>
      invoke(chain.DatabaseGetContentCardsByRoom, {
        room_id: '1.23.0',
        limit: 3,
      }),
    );
  } finally {
    rejectPending(new Error('Test ended'));
    socket.close();
    report.finishedAt = new Date().toISOString();
  }
  return report;
}
