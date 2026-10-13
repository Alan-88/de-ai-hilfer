import assert from "node:assert/strict";
import test from "node:test";
import { Client, InMemoryTransport } from "@modelcontextprotocol/client";
import { DeaiApi } from "./backend.js";
import { buildMcpServer } from "./tools.js";

test("MCP separates follow-up reads, external writes and asynchronous internal generation", async () => {
  const calls: Array<{path:string;method:string;body:unknown}> = [];
  const api = new DeaiApi("https://backend.test/api/v1","test",async (url,init) => {
    calls.push({path:String(url),method:init?.method ?? "GET",body:init?.body ? JSON.parse(String(init.body)) : null});
    return new Response(JSON.stringify(String(url).endsWith("/follow-ups/7") ? {id:7,entry_id:2623,question:"区别？",answer:"旧回答"} : {id:"job",status:"queued"}));
  });
  const server=buildMcpServer(api);const client=new Client({name:"followup-test",version:"1"});
  const [left,right]=InMemoryTransport.createLinkedPair();await server.connect(right);await client.connect(left);
  try {
    const list=await client.listTools();
    assert.equal(list.tools.find(tool=>tool.name==="follow_up_query")?.annotations?.readOnlyHint,true);
    assert.equal(list.tools.find(tool=>tool.name==="follow_up_write")?.annotations?.destructiveHint,true);
    await client.callTool({name:"follow_up_query",arguments:{action:"list",entry_id:2623}});
    assert.ok(calls[0]?.path.endsWith("/agent/follow-ups?entry_id=2623"));
    const payload={action:"create",entry_id:2623,question:"区别？",answer:"**结论**\n\n- 解释",idempotency_key:"external-answer"};
    assert.equal((await client.callTool({name:"follow_up_write",arguments:payload})).isError,undefined);
    assert.deepEqual(calls[1]?.body,{entry_id:2623,question:payload.question,answer:payload.answer,idempotency_key:"external-answer"});
    await client.callTool({name:"follow_up_write",arguments:{action:"edit",follow_up_id:7,question:"区别？",answer:"新版",expected_question:"区别？",expected_answer:"旧回答"}});
    assert.equal(calls[2]?.method,"PUT");
    await client.callTool({name:"follow_up_generate",arguments:{action:"retry",follow_up_id:7,idempotency_key:"retry-7"}});
    assert.equal(calls[3]?.method,"GET");
    assert.deepEqual(calls[4]?.body,{idempotency_key:"retry-7",operations:[{id:"followup",operation:{action:"follow_up_generate",entry_id:2623,question:"区别？",retry_id:7}}]});
    await client.callTool({name:"follow_up_write",arguments:{action:"delete",follow_up_id:7}});
    assert.equal(calls[5]?.method,"DELETE");
    assert.equal((await client.callTool({name:"follow_up_generate",arguments:{action:"create",entry_id:2623,question:"问题",idempotency_key:"bad",provider_name:"partial"}})).isError,true);
    assert.equal(calls.length,6);
  } finally {await client.close();await server.close();}
});
