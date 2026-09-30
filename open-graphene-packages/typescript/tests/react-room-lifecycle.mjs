import { createElement as h, act } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import {
  SwaplockProvider, usePrepareDataRoomCreate, usePrepareDataRoomDelete,
  useSignTransaction, useBroadcastTransaction, useWaitForInclusion, useRoom,
} from '@open-graphene/chain-swaplock-react';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { bytesToHex } from '../graphene-primitives/dist/index.js';
import { stringifyJson } from '../graphene-codec/dist/index.js';
import { RpcRemoteError } from '../graphene-transport/dist/index.js';
globalThis.IS_REACT_ACT_ENVIRONMENT = true;
const assert = (value, message) => { if (!value) throw Error(message); };

/** Real hook pipeline; save only public transaction data, before any broadcast. */
export async function runRoomLifecycle({ client, signer, owner, journal, save }) {
  assert(client.chainId === b.CHAIN.chainId, 'Room E2E requires the pinned Swaplock chain');
  const cache = new QueryClient({ defaultOptions: { queries: { gcTime: Infinity }, mutations: { gcTime: Infinity } } }), element = document.createElement('div');
  document.body.append(element); const root = createRoot(element);
  let api, roomId = journal.roomId;
  const check = async (name, ok) => { assert(ok, name); journal.checks.push({ name, status: 'passed' }); await save(); };
  function Scene() {
    const create = usePrepareDataRoomCreate({ maxFee: 300000n });
    const remove = usePrepareDataRoomDelete({ maxFee: 300000n });
    const sign = useSignTransaction(signer), broadcast = useBroadcastTransaction(), inclusion = useWaitForInclusion();
    const room = useRoom(b.DataRoomId(roomId ?? '1.23.0'), { enabled: Boolean(roomId), live: true });
    api = { create, remove, sign, broadcast, inclusion, room }; return null;
  }
  async function render() { await act(async () => root.render(h(QueryClientProvider, { client: cache }, h(SwaplockProvider, { client }, h(Scene))))); }
  async function mutate(action) { let result; await act(async () => { result = await action(); }); return result; }
  async function waitForSnapshot(predicate) {
    for (let attempt = 0; attempt < 300; attempt++) {
      if (api.room.isError) throw api.room.error;
      if (api.room.live.status === 'error') throw api.room.live.error;
      if (api.room.isSuccess && api.room.live.status === 'live' && predicate(api.room.data)) return;
      await act(async () => { await new Promise(resolve => setTimeout(resolve, 100)); });
    }
    throw Error('Timed out waiting for live room snapshot');
  }
  async function send(label, prepare) {
    const prior = journal.transactions.find(entry => entry.label === label);
    if (prior) {
      if (prior.status === 'included') return prior;
      throw Error('Unresolved prior transaction; inspect chain state before retry: ' + prior.id);
    }
    const prepared = await mutate(prepare);
    const signed = await mutate(() => api.sign.mutateAsync(prepared));
    const native = await client.rpc.request('database', 'get_transaction_hex', [signed.toJSON()]);
    await check(label + ': signed FC matches native C++', native === bytesToHex(b.encodeSignedTransaction(signed.transaction)));
    await check(label + ': node verifies authority', await client.rpc.request('database', 'verify_authority', [signed.toJSON()]) === true);
    const entry = { label, id: signed.id, status: 'prepared', startBlock: prepared.startBlock, wire: stringifyJson(signed.toJSON()) };
    journal.transactions.push(entry); await save();
    try { await mutate(() => api.broadcast.mutateAsync(signed)); }
    catch (error) { entry.status = error instanceof RpcRemoteError ? 'rejected' : 'outcome_unknown'; await save(); throw error; }
    entry.status = 'submitted'; await save();
    const included = await mutate(() => api.inclusion.mutateAsync(signed));
    Object.assign(entry, included, { status: 'included' }); await save();
    return entry;
  }
  async function cleanup() {
    if (!roomId) return;
    await send('delete room', () => api.remove.mutateAsync({ caller: owner.id, room: b.DataRoomId(roomId) }));
    const room = await client.database.getDataRoomById({ room_id: b.DataRoomId(roomId) });
    journal.cleanupConfirmed = room === null; await save();
    assert(journal.cleanupConfirmed, 'Room cleanup was not confirmed');
  }
  try {
    await render();
    const created = await send('create room', () => api.create.mutateAsync({
      owner: owner.id, name: journal.roomName, description: 'Public React SDK E2E fixture',
      subject: [0, {}], room_key: 'public-e2e-fixture-envelope', extensions: { write_policy: 1 },
    }));
    if (!roomId) {
      const block = await client.database.getBlock({ block_num: created.blockNumber });
      const result = block?.transactions[created.transactionIndex]?.operation_results[0];
      assert(result?.[0] === 1, 'Create transaction did not return a room ID');
      roomId = b.DataRoomId(result[1]); journal.roomId = roomId; await save();
    }
    await render();
    // A completed journal can be inspected/rerun without submitting any transaction again.
    if (!journal.cleanupConfirmed) {
      await waitForSnapshot(room => room?.id === roomId && room.name === journal.roomName);
      await check('React live hook observed created room', true);
      await cleanup();
    }
    await waitForSnapshot(room => room === null);
    await check('React live hook observed deletion', true);
    journal.status = 'passed'; journal.finishedAt = new Date().toISOString(); await save();
    return { roomId, transactions: journal.transactions.map(({ id, blockNumber }) => ({ id, blockNumber })), cleanupConfirmed: journal.cleanupConfirmed };
  } finally {
    // A known included create is cleaned up even if a subsequent assertion fails.
    // Unresolved submissions are left journaled for manual reconciliation.
    try {
      if (roomId && !journal.cleanupConfirmed && !journal.transactions.some(t => t.label === 'delete room' && t.status !== 'included')) await cleanup();
    } finally { await act(async () => root.unmount()); cache.clear(); element.remove(); }
  }
}
