// Test fixture setup only: native node FC serialization, NOT generated TS FC.
import { readFile, writeFile } from 'node:fs/promises';
import {
  stringifyJson,
  unknownValue,
} from '../../graphene-codec/dist/index.js';
import * as b from '../../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
import { SwaplockClient } from '../../graphene-chain-swaplock/graphene-chain-swaplock-api/dist/index.js';
import { WifSigner, encodePublicKey } from '../../graphene-fc/dist/signing.js';
import { transactionDigest, sha256 } from '../../graphene-fc/dist/index.js';
import {
  bytesToHex,
  hexToBytes,
  parseTimePointSec,
} from '../../graphene-primitives/dist/index.js';

export async function fixtureWriter(genesisPath, journalPath) {
  const client = await SwaplockClient.connect(
    'wss://node01.swaplock.chainpool.online:8090',
    {
      expectedChainId: b.CHAIN.chainId,
    },
  );
  const genesis = JSON.parse(await readFile(genesisPath, 'utf8'));
  const record = genesis.initial_accounts.find((a) => a.name === 'swaplock');
  const signer = new WifSigner(record.active_key_full.wif_priv_key);
  const [account] = await client.rpc.invoke(b.DatabaseGetAccounts, {
    account_names_or_ids: ['swaplock'],
    subscribe: false,
  });
  const key = encodePublicKey(signer.publicKey, b.CHAIN.publicKeyPrefix);
  if (
    (account.active.key_auths.find(([k]) => k === key)?.[1] ?? 0) <
    account.active.weight_threshold
  )
    throw new Error('Fixture signer lacks active authority');
  let journal;
  try {
    journal = JSON.parse(await readFile(journalPath, 'utf8'));
  } catch (e) {
    if (e.code !== 'ENOENT') throw e;
    journal = {
      startedAt: new Date().toISOString(),
      purpose:
        'test data setup using native node FC serializer; not TS FC coverage',
      transactions: [],
    };
  }
  const save = () =>
    writeFile(journalPath, stringifyJson(unknownValue.encode(journal)) + '\n');
  return {
    client,
    account,
    async send(operation, label) {
      const prior = journal.transactions.find((t) => t.label === label);
      if (prior) {
        if (prior.status === 'included' && prior.operationResults?.length)
          return b.OperationResultCodec.decode(prior.operationResults[0]);
        throw new Error(
          'Existing fixture attempt needs reconciliation; refusing duplicate submission',
        );
      }
      const checked = b.OperationCodec.decode(
        b.OperationCodec.encode(operation),
      );
      const [fee] = await client.rpc.invoke(b.DatabaseGetRequiredFees, {
        ops: [checked],
        asset_symbol_or_id: '1.3.0',
      });
      if (!fee || Array.isArray(fee) || fee.amount > 100000000n)
        throw new Error('Fixture fee exceeds 1000 BTS cap');
      const head = await client.rpc.invoke(
        b.DatabaseGetDynamicGlobalProperties,
        {},
      );
      const tx = {
        ref_block_num: head.head_block_number & 0xffff,
        ref_block_prefix: new DataView(
          head.head_block_id.buffer,
          head.head_block_id.byteOffset,
          20,
        ).getUint32(4, true),
        expiration: new Date((parseTimePointSec(head.time) + 120) * 1000)
          .toISOString()
          .slice(0, 19),
        operations: [[checked[0], { ...checked[1], fee }]],
        extensions: [],
      };
      const wire = b.TransactionCodec.encode(tx);
      const hex = await client.rpc.request(
        'database',
        'get_transaction_hex_without_sig',
        [wire],
      );
      const bytes = hexToBytes(hex);
      const id = bytesToHex(sha256(bytes).slice(0, 20));
      const signature = await signer.signDigest(
        transactionDigest(b.CHAIN.chainId, bytes),
      );
      const signed = b.SignedTransactionCodec.encode({
        ...tx,
        signatures: [signature],
      });
      if (
        (await client.rpc.request('database', 'verify_authority', [signed])) !==
        true
      )
        throw new Error('Fixture authority verification failed');
      const entry = {
        label,
        id,
        feeUnits: fee.amount.toString(),
        status: 'prepared',
      };
      journal.transactions.push(entry);
      await save();
      await client.rpc.invoke(b.NetworkBroadcastBroadcastTransaction, {
        trx: signed,
      });
      entry.status = 'submitted';
      await save();
      const deadline = Date.now() + 45000;
      let blockNumber = head.head_block_number + 1;
      while (Date.now() < deadline) {
        const current = await client.rpc.invoke(
          b.DatabaseGetDynamicGlobalProperties,
          {},
        );
        for (; blockNumber <= current.head_block_number; blockNumber++) {
          const block = await client.rpc.invoke(b.DatabaseGetBlock, {
            block_num: blockNumber,
          });
          for (const [
            transactionIndex,
            candidate,
          ] of block.transactions.entries()) {
            if (
              candidate.ref_block_num !== tx.ref_block_num ||
              candidate.ref_block_prefix !== tx.ref_block_prefix ||
              candidate.expiration !== tx.expiration ||
              candidate.operations[0]?.[0] !== checked[0]
            )
              continue;
            const bare = {
              ref_block_num: candidate.ref_block_num,
              ref_block_prefix: candidate.ref_block_prefix,
              expiration: candidate.expiration,
              operations: candidate.operations,
              extensions: candidate.extensions,
            };
            const candidateHex = await client.rpc.request(
              'database',
              'get_transaction_hex_without_sig',
              [b.TransactionCodec.encode(bare)],
            );
            if (
              bytesToHex(sha256(hexToBytes(candidateHex)).slice(0, 20)) === id
            ) {
              const history = await client.rpc.invoke(
                b.HistoryGetAccountHistory,
                {
                  account_name_or_id: account.id,
                  stop: '1.11.0',
                  limit: 100,
                  start: '1.11.0',
                },
              );
              const applied = history.find(
                (op) =>
                  op.block_num === blockNumber &&
                  op.trx_in_block === transactionIndex &&
                  op.op[0] === checked[0] &&
                  op.op_in_trx === 0,
              );
              if (!applied)
                throw new Error(
                  'Included fixture operation result missing from account history',
                );
              Object.assign(entry, {
                status: 'included',
                blockNumber,
                operationResults: [
                  b.OperationResultCodec.encode(applied.result),
                ],
              });
              await save();
              return applied.result;
            }
          }
        }
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
      throw new Error(
        `Fixture transaction ${id} not observed; never auto-rebroadcast`,
      );
    },
    async close() {
      signer.dispose();
      client.close();
      journal.finishedAt = new Date().toISOString();
      await save();
    },
  };
}
