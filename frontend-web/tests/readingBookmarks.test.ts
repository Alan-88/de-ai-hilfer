import assert from "node:assert/strict";
import test from "node:test";
// @ts-expect-error Node's strip-types test runner requires the source extension.
import { insertReadingBookmark } from "../src/lib/readingBookmarks.ts";

test("marks only the selected repeated occurrence", () => {
  const body = "[Reisen]( Reise ) oder Reisen?";
  const segments = [
    { kind: "bookmark" as const, occurrence_index: 0, surface: "Reisen", form_of: "Reise", lexeme_id: 1, entry_id: 2 },
    { kind: "text" as const, text: " oder Reisen?" },
  ];

  assert.equal(
    insertReadingBookmark(body, segments, { segmentIndex: 1, start: 6, end: 12, surface: "Reisen" }, "Reise"),
    "[Reisen]( Reise ) oder [Reisen](Reise)?",
  );
});

test("rejects a stale selection", () => {
  assert.throws(
    () => insertReadingBookmark("Heute", [{ kind: "text", text: "Heute" }], {
      segmentIndex: 0,
      start: 0,
      end: 5,
      surface: "Morgen",
    }, "heute"),
    /已变化/,
  );
});
