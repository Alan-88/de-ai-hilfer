import test from "node:test";
import assert from "node:assert/strict";

test("isolated HTTP API rejects unauthorized/stale writes and preserves old answers on async failure", {skip:!process.env.DEAI_FOLLOWUP_TEST_URL}, async () => {
  const base=process.env.DEAI_FOLLOWUP_TEST_URL;
  assert.match(base,/^http:\/\/127\.0\.0\.1:18001\/api\/v1$/);
  assert.match(process.env.DEAI_BATCH_TEST_DATABASE_URL ?? "",/\/deai_test$/);
  const request=(path,options={})=>fetch(base+path,{...options,headers:{authorization:"Bearer isolated","content-type":"application/json"}});
  assert.equal((await fetch(base+"/agent/follow-ups?entry_id=1")).status,401);
  const page=await (await request("/entries?q=Wahl")).json();const entry=page.items[0].entry_id;
  const payload={entry_id:entry,question:"临时测试",answer:"**原回答**\n\n- 保留",idempotency_key:"http-followup-test"};
  const created=await request("/agent/follow-ups",{method:"POST",body:JSON.stringify(payload)});assert.equal(created.status,201);
  const original=await created.json();
  assert.equal((await (await request("/agent/follow-ups",{method:"POST",body:JSON.stringify(payload)})).json()).id,original.id);
  assert.equal((await request("/agent/follow-ups",{method:"POST",body:JSON.stringify({...payload,answer:"其他"})})).status,409);
  assert.equal((await request(`/agent/follow-ups/${original.id}`,{method:"PUT",body:JSON.stringify({question:"编辑",answer:"新版",expected_question:"错误旧值",expected_answer:original.answer})})).status,409);
  const edited=await request(`/agent/follow-ups/${original.id}`,{method:"PUT",body:JSON.stringify({question:"编辑",answer:"新版\n\n- 行",expected_question:original.question,expected_answer:original.answer})});assert.equal(edited.status,200);
  const submitted=await request("/agent/batches",{method:"POST",body:JSON.stringify({idempotency_key:"http-failed-retry",operations:[{id:"retry",operation:{action:"follow_up_generate",entry_id:entry,question:"编辑",retry_id:original.id}}]})});
  assert.equal(submitted.status,202);const job=await submitted.json();
  let finished;
  for(let i=0;i<80;i++) {finished=await(await request(`/agent/batches/${job.id}`)).json();if(finished.status==="completed_with_errors")break;await new Promise(resolve=>setTimeout(resolve,100));}
  assert.equal(finished.status,"completed_with_errors");assert.equal(finished.steps[0].status,"failed");
  const kept=await(await request(`/agent/follow-ups/${original.id}`)).json();assert.equal(kept.answer,"新版\n\n- 行");
  assert.equal((await request(`/agent/follow-ups/${original.id}`,{method:"DELETE"})).status,200);
  assert.equal((await request(`/agent/follow-ups/${original.id}`)).status,404);
});
