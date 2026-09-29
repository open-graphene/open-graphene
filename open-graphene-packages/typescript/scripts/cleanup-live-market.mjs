// Cancel only the order created by seed-live-market. Other fixtures remain for read tests.
import {readFile,writeFile} from 'node:fs/promises';
import {fixtureWriter} from '../tests/live/fixture-writer.mjs';
import * as b from '../graphene-chain-swaplock/graphene-chain-swaplock-bindings/dist/index.js';
const [genesisPath,path]=process.argv.slice(2);
if(!genesisPath||!path)throw new Error('Provide genesis and fixture JSON paths');
const fixture=JSON.parse(await readFile(path,'utf8'));
const writer=await fixtureWriter(genesisPath,path+'.journal.json');
try {
 const [order]=await writer.client.rpc.invoke(b.DatabaseGetObjects,{ids:[fixture.orderId],subscribe:false});
 if(order) await writer.send(b.operation.limit_order_cancel({fee:{amount:0n,asset_id:b.AssetId('1.3.0')},fee_paying_account:writer.account.id,order:fixture.orderId,extensions:[]}),'cancel fixture resting order');
 const [remaining]=await writer.client.rpc.invoke(b.DatabaseGetObjects,{ids:[fixture.orderId],subscribe:false});
 if(remaining!==null)throw new Error('Fixture order still exists');
 fixture.orderClosedAt=new Date().toISOString();
 await writeFile(path,JSON.stringify(fixture,null,2)+'\n');
 console.log('Fixture resting order is closed');
}finally{await writer.close()}
