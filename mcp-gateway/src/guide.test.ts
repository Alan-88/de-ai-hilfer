import { readFile } from "node:fs/promises";
import test from "node:test";
import assert from "node:assert/strict";

import { DEAI_GUIDES, DEAI_GUIDE_TOPICS, LEARNING_GUIDE, SERVER_INSTRUCTIONS } from "./guide.js";

test("MCP learning guide stays aligned with the repository CLI", async () => {
  const cli = await readFile(new URL("../../tools/deai", import.meta.url), "utf8");
  const match = cli.match(/const LEARNING_GUIDE = String\.raw`([\s\S]*?)`;\n/);
  assert.ok(match, "tools/deai must keep an extractable LEARNING_GUIDE constant");
  assert.equal(LEARNING_GUIDE, match[1]);
});

test("MCP guidance exposes every topic and reuses the V3 learning guide", () => {
  assert.deepEqual(Object.keys(DEAI_GUIDES), [...DEAI_GUIDE_TOPICS]);
  assert.equal(DEAI_GUIDES.learning_v3.details, LEARNING_GUIDE);
  assert.match(SERVER_INSTRUCTIONS, /article-first German learning system/);
  assert.match(SERVER_INSTRUCTIONS, /lexeme_id and entry_id are not interchangeable/);
  for (const topic of DEAI_GUIDE_TOPICS) {
    const guide = DEAI_GUIDES[topic];
    assert.equal(guide.topic, topic);
    assert.ok(guide.summary);
    assert.ok(guide.concepts.length);
    assert.ok(guide.workflow.length);
    assert.ok(guide.safety_rules.length);
    assert.ok(guide.related_tools.length);
  }
});
