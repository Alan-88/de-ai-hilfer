import assert from "node:assert/strict";
import test from "node:test";
import { Client, InMemoryTransport } from "@modelcontextprotocol/client";
import { DeaiApi } from "./backend.js";
import { buildMcpServer } from "./tools.js";
import { batchSubmission } from "./batch.js";

test("batch schema permits mixed actions and references but rejects arbitrary requests", () => {
  assert.equal(batchSubmission.parse({idempotency_key:"mixed",operations:[
    {id:"folder",operation:{action:"folder_create",name:"German"}},
    {id:"article",operation:{action:"article_import",title:"Text",body:"Hallo",folder_id:{step:"folder",path:"/folder_id"}}},
  ]}).operations.length,2);
  assert.throws(() => batchSubmission.parse({idempotency_key:"bad",operations:[{id:"http",operation:{action:"http",url:"https://example.test"}}]}));
  assert.throws(() => batchSubmission.parse({idempotency_key:"bad",operations:[{id:"folder",operation:{action:"folder_create",name:"German",unknown:true}}]}));
});

test("MCP initialization submits one async job; progress is read-only and retries preserve the task ID", async () => {
  const calls: Array<{path:string,method:string,body:unknown}> = [];
  const api = new DeaiApi("https://backend.test/api/v1","test",async (url,init) => {
    calls.push({path:String(url),method:init?.method ?? "GET",body:init?.body ? JSON.parse(String(init.body)) : null});
    return new Response(JSON.stringify({id:"job-1",status:"queued",steps:[]}),{status:202});
  });
  const server = buildMcpServer(api);
  const client = new Client({name:"batch-test",version:"1"});
  const [clientTransport,serverTransport] = InMemoryTransport.createLinkedPair();
  await server.connect(serverTransport);
  await client.connect(clientTransport);
  try {
    const listed = await client.listTools();
    assert.equal(listed.tools.find(t=>t.name==="batch_query")?.annotations?.readOnlyHint,true);
    const submitted = await client.callTool({name:"lexeme_initialize",arguments:{lexeme_ids:[12,34],idempotency_key:"confirmed-vocabulary"}});
    assert.equal(submitted.isError,undefined);
    assert.equal(calls.length,1);
    assert.ok(calls[0]);
    assert.ok(calls[0].path.endsWith("/agent/batches"));
    assert.deepEqual(calls[0].body,{idempotency_key:"confirmed-vocabulary",operations:[{id:"initialize",operation:{action:"lexeme_initialize",lexeme_ids:[12,34]}}]});
    await client.callTool({name:"batch_query",arguments:{action:"get",batch_id:"job-1"}});
    assert.ok(calls[1]);
    assert.equal(calls[1].method,"GET");
    assert.ok(calls[1].path.endsWith("/agent/batches/job-1"));
    await client.callTool({name:"batch_retry",arguments:{batch_id:"job-1"}});
    assert.ok(calls[2]);
    assert.ok(calls[2].path.endsWith("/agent/batches/job-1/retry"));
    assert.equal(calls[2].method,"POST");
    const rejected = await client.callTool({name:"lexeme_initialize",arguments:{lexeme_ids:[12],provider_name:"partial"}});
    assert.equal(rejected.isError,true);
    assert.equal(calls.length,3);
    await client.callTool({name:"batch_submit",arguments:{idempotency_key:"folder-plan",operations:[{id:"folder",operation:{action:"folder_create",name:"German"}}]}});
    assert.ok(calls[3]);
    assert.ok(calls[3].path.endsWith("/agent/batches"));
  } finally { await client.close(); await server.close(); }
});
