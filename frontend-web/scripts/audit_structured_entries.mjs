// Read-only API snapshot; compare the same stored inputs after changing the resolver.
// Run from frontend-web: node scripts/audit_structured_entries.mjs capture|compare SNAPSHOT [REPORT]
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { createServer } from "vite";
import { setTimeout } from "node:timers/promises";

const [mode, snapshotPath, reportPath = `${snapshotPath}.report.json`] = process.argv.slice(2);
assert(["capture", "compare"].includes(mode) && snapshotPath, "capture|compare SNAPSHOT [REPORT]");
const server = await createServer({ server: { middlewareMode: true }, appType: "custom" });
try {
  const { resolveStructuredAnalysis } = await server.ssrLoadModule("/src/lib/analysis/structuredAnalysis.ts");
  const resolveEntry = (entry) => resolveStructuredAnalysis(entry.analysis_markdown, entry.structured_analysis, entry.query_text);
  if (mode === "capture") {
    const baseUrl = process.env.DEAI_API_URL || "https://de.alan-studio.uk/api/v1";
    async function get(path) {
      for (let attempt = 0; ; attempt++) {
        try {
          const response = await fetch(`${baseUrl}${path}`, { signal: AbortSignal.timeout(30000) });
          if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
          return await response.json();
        } catch (error) {
          if (attempt === 2) throw error;
          await setTimeout(500 * (attempt + 1));
        }
      }
    }
    const list = await get("/entries/all");
    assert(Array.isArray(list), "entries/all must return an array");
    const entries = [];
    const failures = [];
    let cursor = 0;
    await Promise.all(Array.from({ length: 6 }, async () => {
      while (cursor < list.length) {
        const item = list[cursor++];
        try {
          const entry = await get(`/entries/${item.entry_id}`);
          assert.equal(entry.entry_id, item.entry_id);
          entries.push({ entry, before: resolveEntry(entry) });
        } catch (error) {
          failures.push({ ...item, error: String(error) });
        }
        if ((entries.length + failures.length) % 200 === 0) console.log(`read ${entries.length + failures.length}/${list.length}`);
      }
    }));
    entries.sort((a, b) => a.entry.entry_id - b.entry.entry_id);
    await writeFile(snapshotPath, JSON.stringify({ createdAt: new Date().toISOString(), baseUrl, total: list.length, failures, entries }));
    console.log(JSON.stringify({ total: list.length, captured: entries.length, failures }));
    if (failures.length) process.exitCode = 1;
  } else {
    const snapshot = JSON.parse(await readFile(snapshotPath, "utf8"));
    const changes = [];
    const regressions = [];
    let unchanged = 0;
    let previouslyStructured = 0;
    for (const { entry, before } of snapshot.entries) {
      const after = resolveEntry(entry);
      if (entry.structured_analysis) {
        const stored = resolveStructuredAnalysis("", entry.structured_analysis, entry.query_text);
        for (const key of ["headword", "phonetic", "meanings", "grammarBranches"]) {
          if (!contains(after[key], stored[key])) regressions.push({ id: entry.entry_id, key, reason: "stored core lost" });
        }
      }
      if (before.sourceType === "structured") previouslyStructured++;
      if (JSON.stringify(before) === JSON.stringify(after)) { unchanged++; continue; }
      const fields = Object.keys(after).filter((key) => JSON.stringify(before[key]) !== JSON.stringify(after[key]));
      changes.push({ id: entry.entry_id, word: entry.query_text, fields, before, after });
      // Previously accepted structured data must survive intact; additions may repair omissions.
      if (before.sourceType === "structured") {
        for (const key of ["headword", "phonetic", "meanings", "grammarBranches", "rawMarkdown"]) {
          if (JSON.stringify(before[key]) !== JSON.stringify(after[key])) regressions.push({ id: entry.entry_id, key });
        }
        for (const key of ["usageModules", "collocations", "examples", "grammarRows", "deepInsights"]) {
          for (const value of before[key]) {
            // Partial records may gain missing fields, but existing values must remain.
            const preserved = after[key].some((candidate) => contains(candidate, value));
            if (!preserved) regressions.push({ id: entry.entry_id, key, value });
          }
        }
        for (const bucket of ["family", "synonyms", "antonyms"]) {
          for (const value of before.wordNetwork[bucket]) {
            if (!after.wordNetwork[bucket].some((candidate) => contains(candidate, value))) regressions.push({ id: entry.entry_id, bucket, value });
          }
        }
      }
    }
    const summary = { total: snapshot.total, captured: snapshot.entries.length, failures: snapshot.failures.length, previouslyStructured, unchanged, changed: changes.length, regressions: regressions.length };
    await writeFile(reportPath, JSON.stringify({ summary, regressions, changes }, null, 2));
    console.log(JSON.stringify(summary));
    assert.equal(regressions.length, 0, "Previously valid content changed; inspect report");
    assert.equal(snapshot.failures.length, 0, "Snapshot is incomplete");
  }
} finally {
  await server.close();
}

function contains(actual, expected) {
  if (expected === null || expected === "") return true;
  if (typeof expected !== "object") return actual === expected;
  return actual != null && Object.entries(expected).every(([key, value]) => contains(actual[key], value));
}
