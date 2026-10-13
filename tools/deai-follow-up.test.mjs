import test from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { once } from "node:events";
import { spawn } from "node:child_process";

test("CLI preserves external Markdown, protects edits and queues retries without waiting for the model", async () => {
  const calls=[];
  const server=createServer(async(request,response)=>{
    let body="";for await(const chunk of request) body+=chunk;
    calls.push({path:request.url,method:request.method,body:body ? JSON.parse(body) : null});
    response.setHeader("content-type","application/json");
    response.end(JSON.stringify({id:7,entry_id:2623,question:"区别？",answer:"旧回答"}));
  }).listen(0,"127.0.0.1");await once(server,"listening");
  const run=async(args,input="")=>{
    const child=spawn(process.execPath,[new URL("./deai",import.meta.url).pathname,"follow-up",...args,"--url",`http://127.0.0.1:${server.address().port}/api/v1`,"--token","test"],{stdio:["pipe","pipe","pipe"]});
    let error="";child.stderr.on("data",chunk=>error+=chunk);child.stdout.resume();child.stdin.end(input);
    const [code]=await once(child,"close");return {code,error};
  };
  try {
    assert.equal((await run(["list","2623"])).code,0);
    const payload={entry_id:2623,question:"区别？",answer:"**结论**\n\n- 解释"};
    assert.equal((await run(["create","--stdin","--yes","--idempotency-key","manual"],JSON.stringify(payload))).code,0);
    assert.deepEqual(calls[1].body,{...payload,idempotency_key:"manual"});
    assert.equal((await run(["edit","7","--stdin","--yes"],JSON.stringify({...payload,expected_question:"区别？",expected_answer:"旧回答"}))).code,0);
    assert.equal(calls[2].method,"PUT");
    assert.equal((await run(["retry","7","--yes","--idempotency-key","retry"])).code,0);
    assert.deepEqual(calls[4].body,{idempotency_key:"retry",operations:[{id:"followup",operation:{action:"follow_up_generate",entry_id:2623,question:"区别？",retry_id:7}}]});
    assert.equal((await run(["delete","7","--yes"])).code,0);
    assert.equal(calls[5].method,"DELETE");
    assert.notEqual((await run(["delete","7"])).code,0);
    assert.notEqual((await run(["edit","7","--stdin","--yes"],JSON.stringify(payload))).code,0);
    assert.equal(calls.length,6);
  } finally {server.close();}
});
