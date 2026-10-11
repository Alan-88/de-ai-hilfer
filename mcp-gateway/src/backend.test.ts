import test from "node:test";
import assert from "node:assert/strict";

import { DeaiApi, type FetchImplementation } from "./backend.js";

test("read requests retry transient network failures twice", async () => {
  let calls = 0;
  const fakeFetch: FetchImplementation = async () => {
    calls += 1;
    if (calls < 3) throw new TypeError("fetch failed", { cause: { code: "ECONNRESET", message: "reset" } });
    return new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  };
  const api = new DeaiApi("https://example.test/api/v1", "token", fakeFetch);
  assert.deepEqual(await api.get("/agent/learning"), { ok: true });
  assert.equal(calls, 3);
});

test("write requests are never retried after a network failure", async () => {
  let calls = 0;
  const fakeFetch: FetchImplementation = async () => {
    calls += 1;
    throw new TypeError("fetch failed", { cause: { code: "ECONNRESET", message: "reset" } });
  };
  const api = new DeaiApi("https://example.test/api/v1", "token", fakeFetch);
  await assert.rejects(api.post("/agent/learning/123/reset"), /after 1 attempt/);
  assert.equal(calls, 1);
});
