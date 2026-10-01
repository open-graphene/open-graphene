// Generate public synthetic operation vectors; Rust/native RPC are independent oracles.
import { readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const rich = process.argv.includes('--rich');
const chain = process.argv.includes('--bitshares') ? 'bitshares' : 'swaplock';
const b = await import(
  '../graphene-chain-' +
    chain +
    '/graphene-chain-' +
    chain +
    '-bindings/dist/index.js'
);
import { bytesToHex } from '../graphene-primitives/dist/index.js';
import { stringifyJson } from '../graphene-codec/dist/index.js';
import { RpcClient } from '../graphene-transport/dist/index.js';
const root = fileURLToPath(new URL('../../../', import.meta.url));
const spec = JSON.parse(
  await readFile(
    new URL(
      '../../rust/graphene-chain-' +
        chain +
        '/graphene-chain-' +
        chain +
        '-spec/dist/' +
        chain +
        '.open-graphene.json',
      import.meta.url,
    ),
  ),
);
const defs = new Map(
  [...spec.structs, ...spec.operations].map((s) => [s.name, s]),
);
const variants = new Map(spec.staticVariants.map((s) => [s.name, s]));
const publicKey = 'BTS6MRyAjQq8ud7hVNYcfnVPJqcVpscN5So8BhtHuGYqET5GDW5CV';
function value(t, depth = 0) {
  if (depth > 25) throw Error('Recursive fixture');
  switch (t.kind) {
    case 'bool':
      return rich;
    case 'uint8':
    case 'uint16':
    case 'uint32':
    case 'int32':
    case 'unsigned_varint':
      return rich ? 7 : 0;
    case 'uint64':
    case 'uint128':
    case 'int64':
      return rich ? '9007199254740993' : '0';
    case 'string':
      return rich ? 'public-fixture' : '';
    case 'public_key':
      return publicKey;
    case 'object_id':
      return '1.2.100';
    case 'protocol_object_id': {
      const o = spec.objectTypes.find((o) => o.objectType === t.objectType);
      return o.objectSpace + '.' + o.typeId + '.0';
    }
    case 'vote_id':
      return '0:0';
    case 'time_point_sec':
      return '2026-09-29T00:00:00';
    case 'bytes':
      return rich ? '0123abcd' : '';
    case 'fixed_bytes':
    case 'fixed_hex':
      return '00'.repeat(t.bytes);
    case 'signature':
      return '00'.repeat(65);
    case 'optional':
      return rich ? value(t.inner, depth + 1) : undefined;
    case 'void':
      return {};
    case 'vector':
    case 'set':
      return rich ? [value(t.inner, depth + 1)] : [];
    case 'flat_map':
    case 'map':
      return rich && t.key.kind !== 'address'
        ? [[value(t.key, depth + 1), value(t.value, depth + 1)]]
        : [];
    case 'pair':
      return [value(t.first, depth + 1), value(t.second, depth + 1)];
    case 'ref':
      return struct(t.name, depth + 1);
    case 'static_variant_ref': {
      const arm = variants.get(t.name).variants[0];
      return [arm.tag, value(arm.type, depth + 1)];
    }
    default:
      throw Error('Fixture type ' + t.kind);
  }
}
const extensions = new Set(
  [...spec.structs, ...spec.operations]
    .flatMap((s) => s.fields)
    .filter((f) => f.name === 'extensions' && f.type.kind === 'ref')
    .map((f) => f.type.name),
);
const sparse = new Set([
  'data_room_access_extensions',
  'data_room_create_operation_ext',
  'content_card_update_operation_ext',
  'content_card_remove_operation_ext',
]);
function struct(name, depth = 0) {
  if (extensions.has(name) && !sparse.has(name)) return {};
  return Object.fromEntries(
    defs
      .get(name)
      .fields.map((f) => [f.name, value(f.type, depth + 1)])
      .filter(([, v]) => v !== undefined),
  );
}
const vectors = [];
for (const op of spec.operations.filter((o) => !o.isVirtual)) {
  const operation = [op.wireTag, struct(op.name)];
  try {
    vectors.push({
      name: op.name,
      operation,
      hex: bytesToHex(b.encodeOperation(b.OperationCodec.decode(operation))),
    });
  } catch (e) {
    vectors.push({ name: op.name, operation, error: e.message });
  }
}
const oracle = spawnSync(
  root +
    'target/debug/examples/fc_oracle' +
    (chain === 'bitshares' ? '_bitshares' : ''),
  [],
  {
    input:
      vectors
        .map((v) =>
          stringifyJson(
            b.OperationCodec.encode(b.OperationCodec.decode(v.operation)),
          ),
        )
        .join('\n') + '\n',
    encoding: 'utf8',
  },
);
if (oracle.status !== 0) throw Error(oracle.stderr || oracle.error);
const answers = oracle.stdout.trim().split('\n').map(JSON.parse);
let client;
if (process.argv.includes('--live'))
  client = await RpcClient.connect(
    chain === 'swaplock'
      ? 'wss://node01.swaplock.chainpool.online:8090'
      : 'wss://api.dex.trading/',
  );
try {
  for (const [i, v] of vectors.entries()) {
    v.rust = answers[i];
    if (v.hex !== v.rust.hex) {
      console.error(v.name, v.error, v.rust);
      process.exitCode = 1;
    }
    if (client && v.hex) {
      try {
        const tx = {
          ref_block_num: 0,
          ref_block_prefix: 0,
          expiration: '2026-09-29T00:00:00',
          operations: [v.operation],
          extensions: [],
        };
        const native = await client.request(
          'database',
          'get_transaction_hex_without_sig',
          [tx],
        );
        const local = bytesToHex(
          b.encodeTransaction(b.TransactionCodec.decode(tx)),
        );
        v.nativeMatch = native === local;
        if (!v.nativeMatch) {
          v.nativeHex = native;
          process.exitCode = 1;
          console.error('Native mismatch', v.name);
        }
      } catch (e) {
        v.nativeError = e.message;
        process.exitCode = 1;
        console.error('Native error', v.name, e.message.slice(0, 100));
      }
    }
  }
} finally {
  client?.close();
}
await writeFile(
  '/tmp/' + chain + '-fc-parity-vectors' + (rich ? '-rich' : '') + '.json',
  JSON.stringify(
    {
      source:
        'public synthetic inputs, compared with Rust FC and optionally native C++ RPC',
      vectors,
    },
    null,
    2,
  ) + '\n',
);
console.log(
  JSON.stringify(
    {
      operations: vectors.length,
      rustMatched: vectors.filter((v) => v.hex && v.hex === v.rust.hex).length,
      nativeMatched: vectors.filter((v) => v.nativeMatch).length,
      errors: vectors
        .filter((v) => v.error)
        .map((v) => ({ name: v.name, error: v.error })),
    },
    null,
    2,
  ),
);
