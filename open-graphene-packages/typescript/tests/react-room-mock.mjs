import { runRoomLifecycle } from './react-room-lifecycle.mjs';
import { SwaplockClient, transactionId } from '../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { PrivateKey } from '../graphene-fc/dist/wallet.js';
import { transactionDigest } from '../graphene-fc/dist/index.js';
import { verifyDigestCompact } from '../graphene-fc/dist/signing.js';
import { bytesToHex } from '../graphene-primitives/dist/index.js';
import { RpcSubscription, RpcRemoteError } from '../graphene-transport/dist/index.js';
export async function runMockRoomE2E() {
  const signer = PrivateKey.fromSeed(new TextEncoder().encode('Public React room E2E test seed'));
  const owner = { id: b.AccountId('1.2.100') }, roomId = b.DataRoomId('1.23.500');
  let room = null, headNumber = 100, broadcasts = 0, saves = 0;
  const blocks = new Map(), streams = new Set();
  const journal = { chainId: b.CHAIN.chainId, roomName: 'public-mock-room', transactions: [], checks: [] };
  const save = async () => { saves++; };
  const head = () => ({ head_block_number: headNumber, head_block_id: new Uint8Array(20).fill(3), time: new Date().toISOString().slice(0, 19) });
  const unsigned = ({ ref_block_num, ref_block_prefix, expiration, operations, extensions }) => ({ ref_block_num, ref_block_prefix, expiration, operations, extensions });
  const rpc = {
    chainId: b.CHAIN.chainId,
    invoke: async (method, params) => {
      switch (method.method) {
        case 'get_required_fees': return params.ops.map(() => ({ amount: 1n, asset_id: b.AssetId('1.3.0') }));
        case 'get_dynamic_global_properties': return head();
        case 'get_objects': return [];
        case 'get_data_room_by_id': return params.room_id === roomId ? room : null;
        case 'get_block': return blocks.get(params.block_num) ?? null;
        case 'broadcast_transaction': {
          const tx = b.SignedTransactionCodec.decode(params.trx), id = transactionId(unsigned(tx));
          const entry = journal.transactions.find(entry => entry.id === id);
          if (!entry || entry.status !== 'prepared' || !saves) throw Error('Broadcast preceded durable journal');
          broadcasts++;
          const create = tx.operations[0][0] === 78;
          room = create ? { id: roomId, name: journal.roomName } : null;
          blocks.set(++headNumber, { timestamp: head().time, transactions: [{ ...tx, operation_results: [create ? [1, roomId] : [0, {}]] }] });
          for (const stream of streams) stream.push([{ id: '2.1.0', ...head() }]);
          return;
        }
        default: throw Error('Unexpected mocked RPC: ' + method.method);
      }
    },
    request: async (_api, method, args) => {
      const tx = b.SignedTransactionCodec.decode(args[0]);
      if (method === 'get_transaction_hex') return bytesToHex(b.encodeSignedTransaction(tx));
      if (method === 'verify_authority') return verifyDigestCompact(transactionDigest(b.CHAIN.chainId, b.encodeTransaction(unsigned(tx))), tx.signatures[0], signer.publicKey);
      throw Error('Unexpected raw RPC');
    },
    databaseNotices: async () => { const stream = new RpcSubscription(1, () => streams.delete(stream)); streams.add(stream); return stream; },
  };
  const client = new SwaplockClient(rpc);
  try {
    const result = await runRoomLifecycle({ client, signer, owner, journal, save });
    if (broadcasts !== 2 || !result.cleanupConfirmed || streams.size) throw Error('Lifecycle/cleanup failed');
    // Completed journal is reused without repeating either transaction.
    await runRoomLifecycle({ client, signer, owner, journal, save });
    if (broadcasts !== 2 || streams.size) throw Error('Completed journal replay broadcast again');
    const unknown = { ...journal, transactions: [{ label: 'create room', id: 'public-unresolved-id', status: 'outcome_unknown' }], cleanupConfirmed: true };
    let rejected = false;
    try { await runRoomLifecycle({ client, signer, owner, journal: unknown, save }); } catch (error) { rejected = error.message.includes('Unresolved prior transaction'); }
    if (!rejected || broadcasts !== 2 || streams.size) throw Error('Unresolved journal was not refused/cleaned up');
    // Simulate an explicit create rejection. No room or cleanup transaction may be submitted.
    const rejectedJournal = { chainId: b.CHAIN.chainId, roomName: 'rejected-room', transactions: [], checks: [] };
    const realInvoke = rpc.invoke;
    rpc.invoke = async (method, params) => { if (method.method === 'broadcast_transaction') throw new RpcRemoteError({ message: 'fixture rejected' }); return realInvoke(method, params); };
    let explicitRejected = false;
    try { await runRoomLifecycle({ client, signer, owner, journal: rejectedJournal, save }); } catch (error) { explicitRejected = error instanceof RpcRemoteError; }
    if (!explicitRejected || rejectedJournal.transactions[0]?.status !== 'rejected' || broadcasts !== 2 || streams.size) throw Error('Explicit rejection incorrectly retried or leaked a subscription');
    return { createSignBroadcastInclusion: true, liveRoomAndDeletion: true, journalBeforeBroadcast: true, completedReplayNoBroadcast: true, unresolvedRefused: true, rejectionNoRetry: true, cleanup: true };
  } finally { signer.dispose(); for (const stream of streams) stream.close(); }
}
