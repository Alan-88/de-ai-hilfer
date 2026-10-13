import test from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { once } from "node:events";
import { spawn } from "node:child_process";

test("CLI forwards batch submit/get/retry and preserves synchronous initialization", async () => {
  const calls = [];
  const server = createServer(async (request,response) => {
    let body = "";
    for await (const chunk of request) body += chunk;
    calls.push({path:request.url,method:request.method,body:body ? JSON.parse(body) : null});
    assert.equal(request.headers.authorization,"Bearer test-token");
    response.setHeader("content-type","application/json");
    response.end(JSON.stringify({id:"test-job",status:"queued"}));
  }).listen(0,"127.0.0.1");
  await once(server,"listening");
  const url = `http://127.0.0.1:${server.address().port}/api/v1`;
  const run = async (args,input="") => {
    const child = spawn(process.execPath,[new URL("./deai",import.meta.url).pathname,...args,"--url",url,"--token","test-token"],{stdio:["pipe","pipe","pipe"]});
    let output="",error="";
    child.stdout.on("data",chunk=>output+=chunk);
    child.stderr.on("data",chunk=>error+=chunk);
    child.stdin.end(input);
    const [code] = await once(child,"close");
    return {code,output,error};
  };
  try {
    const payload = {idempotency_key:"test-plan",operations:[{id:"folder",operation:{action:"folder_create",name:"German"}}]};
    assert.equal((await run(["batch","submit","--stdin","--yes"],JSON.stringify(payload))).code,0);
    assert.deepEqual(calls[0],{path:"/api/v1/agent/batches",method:"POST",body:payload});
    assert.equal((await run(["batch","get","test-job"])).code,0);
    assert.equal(calls[1].method,"GET");
    assert.equal((await run(["batch","retry","test-job","--yes"])).code,0);
    assert.equal(calls[2].path,"/api/v1/agent/batches/test-job/retry");
    assert.equal((await run(["entry","initialize","12","34","--async","--yes","--idempotency-key","words"])).code,0);
    assert.deepEqual(calls[3].body,{idempotency_key:"words",operations:[{id:"initialize",operation:{action:"lexeme_initialize",lexeme_ids:[12,34]}}]});
    assert.equal((await run(["entry","initialize","12","--yes"])).code,0);
    assert.equal(calls[4].path,"/api/v1/agent/lexemes/12/initialize");
    assert.notEqual((await run(["batch","submit","--stdin"],JSON.stringify(payload))).code,0);
    assert.equal(calls.length,5);
  } finally { server.close(); }
});
