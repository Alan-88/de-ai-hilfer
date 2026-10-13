import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:net";
import { randomUUID } from "node:crypto";
import { fileURLToPath } from "node:url";

test("Agent API responds immediately and recovers a queued batch after a process restart", {skip:!process.env.DEAI_BATCH_TEST_DATABASE_URL}, async () => {
  const database = new URL(process.env.DEAI_BATCH_TEST_DATABASE_URL);
  assert.equal(database.pathname,"/deai_test");
  assert.ok(["127.0.0.1","localhost"].includes(database.hostname));
  const portReservation = createServer().listen(0,"127.0.0.1");
  await once(portReservation,"listening");
  const port = portReservation.address().port;
  await new Promise(resolve=>portReservation.close(resolve));
  const root = fileURLToPath(new URL("../backend/",import.meta.url));
  let child;
  let logs="";
  const start = async () => {
    child = spawn(`${root}target/debug/server`,[],{cwd:root,env:{...process.env,
      DATABASE_URL:database.href,SERVER_HOST:"127.0.0.1",SERVER_PORT:String(port),DEAI_API_TOKEN:"isolated-test",
      OPENAI_API_KEY:"isolated-test",OPENAI_BASE_URL:"http://127.0.0.1:9",RUST_LOG:"error",
    },stdio:["ignore","pipe","pipe"]});
    child.stdout.on("data",chunk=>logs+=chunk); child.stderr.on("data",chunk=>logs+=chunk);
    for(let i=0;i<100;i++) {
      if(child.exitCode!==null) throw Error(logs);
      try { if((await fetch(`http://127.0.0.1:${port}/health`)).ok) return; } catch {}
      await new Promise(resolve=>setTimeout(resolve,50));
    }
    throw Error(`Test backend did not start: ${logs}`);
  };
  const stop = async () => {
    if(child && child.exitCode===null && child.signalCode===null) { const exited=once(child,"exit"); child.kill("SIGTERM"); await exited; }
  };
  const request = (path,options={}) => fetch(`http://127.0.0.1:${port}/api/v1/agent${path}`,{
    ...options,headers:{authorization:"Bearer isolated-test","content-type":"application/json"},
  });
  try {
    await start();
    assert.equal((await fetch(`http://127.0.0.1:${port}/api/v1/agent/batches`)).status,401);
    const bad = await request("/batches",{method:"POST",body:JSON.stringify({idempotency_key:"invalid",operations:[{id:"bad",depends_on:["future"],operation:{action:"folder_create",name:"Bad"}}]})});
    assert.equal(bad.status,400);
    const key = randomUUID();
    const payload={idempotency_key:key,operations:[{id:"folder",operation:{action:"folder_create",name:`Restart ${key}`}}]};
    const began=performance.now();
    const submitted = await request("/batches",{method:"POST",body:JSON.stringify(payload)});
    assert.equal(submitted.status,202);
    const job=await submitted.json();
    assert.equal(job.status,"queued");
    assert.ok(performance.now()-began<1000,"submission must not wait for execution");
    await stop();
    assert.match(job.id,/^[0-9a-f-]{36}$/);
    const persisted = execFileSync("docker",["exec","deai-batch-tests-20261013","psql","-U","postgres","-d","deai_test","-Atc",`SELECT status FROM agent_batches WHERE id='${job.id}'`],{encoding:"utf8"}).trim();
    assert.equal(persisted,"queued","task must still be pending at process termination");
    await start();
    let finished;
    for(let i=0;i<100;i++) {
      finished=await (await request(`/batches/${job.id}`)).json();
      if(finished.status==="succeeded") break;
      await new Promise(resolve=>setTimeout(resolve,50));
    }
    assert.equal(finished.status,"succeeded");
    assert.ok(finished.steps[0].result.folder_id>0);
    assert.equal((await (await request("/batches",{method:"POST",body:JSON.stringify(payload)})).json()).id,job.id);
    const changed={...payload,on_error:"stop"};
    assert.equal((await request("/batches",{method:"POST",body:JSON.stringify(changed)})).status,409);
  } finally { await stop(); }
});
