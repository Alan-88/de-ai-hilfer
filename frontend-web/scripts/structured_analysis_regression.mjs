// Run from frontend-web: node scripts/structured_analysis_regression.mjs
import assert from "node:assert/strict";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true }, appType: "custom" });
try {
  const { resolveStructuredAnalysis: resolve, parseAnalysisMarkdown: parse } = await server.ssrLoadModule("/src/lib/analysis/structuredAnalysis.ts");
  const core = {
    headword: "gar", phonetic: "/ɡaːr/",
    meanings: [{ part_of_speech: "adverb", chinese: "完全", english: "at all" }],
    grammar_branches: [{ pos: "adverb", selector: "gar", meanings: [{ zh: "完全", en: "at all" }], grammar: {} }],
  };
  const markdown = "#### 应用与例句\n\ngar nicht\n用法解析：加强否定\n场景例句：Das stimmt gar nicht.\n例句翻译：这完全不对。\n\n#### 词汇网络\n\n**词族**\n* [garen] (Verb, 烹饪, cook)\n\n**同义词**\n* [überhaupt] (Adv, 根本, at all)\n\n**反义词**\n* 无高价值项\n\n#### 深度解析与避坑\n\n* **语义演变**\n词源在这一段，不需要独立词源标题。\n    1. **内层条目**：保留在正文。\n\n* **辨析**\n第二条解释。";
  const result = resolve(markdown, core, "gar");
  assert.equal(result.sourceType, "structured");
  assert.equal(result.phonetic, core.phonetic);
  assert.deepEqual(result.grammarBranches, core.grammar_branches);
  assert.equal(result.meanings[0].chinese, "完全");
  assert.equal(result.deepInsights.length, 2);
  assert.match(result.deepInsights[0].plain, /内层条目/);
  assert.equal(result.wordNetwork.family[0].term, "garen");
  assert.equal(result.wordNetwork.family[0].chinese, "烹饪");
  assert.equal(result.wordNetwork.antonyms.length, 0);
  assert.equal(result.usageModules.length, 1);
  assert.equal(result.examples.length, 0);
  assert.equal(result.collocations.length, 0);

  const partial = resolve(markdown, {
    ...core,
    usage_modules: [{ title: "gar nicht", explanation: "保留自己的解释", example_de: "保留自己的例句", example_zh: "" }],
    deep_insights: [{ title: "语义演变", content_markdown: "保留自己的原文" }],
    word_network: { family: [{ term: "garen", chinese: "保留自己的释义" }], synonyms: [], antonyms: [] },
  });
  assert.equal(partial.usageModules.length, 1);
  assert.equal(partial.usageModules[0].explanation, "保留自己的解释");
  assert.equal(partial.usageModules[0].example.de, "保留自己的例句");
  assert.equal(partial.usageModules[0].example.zh, "这完全不对。");
  assert.equal(partial.deepInsights.length, 2);
  assert.equal(partial.deepInsights[0].plain, "保留自己的原文");
  assert.equal(partial.wordNetwork.family[0].chinese, "保留自己的释义");
  assert.equal(partial.wordNetwork.synonyms.length, 1);

  const full = {
    ...core,
    usage_modules: [{ title: "gar nicht", explanation: "原文", example_de: "Beispiel", example_zh: "例句" }],
    deep_insights: [{ title: "语义演变", content_markdown: "原文1" }, { title: "辨析", content_markdown: "原文2" }],
    word_network: { family: [{ term: "garen" }], synonyms: [{ term: "überhaupt" }], antonyms: [] },
  };
  const before = resolve("", full);
  assert.deepEqual(resolve(markdown, full), { ...before, rawMarkdown: markdown });
  assert.equal(resolve("", { grammar_branches: core.grammar_branches }).grammarBranches.length, 1);
  assert.equal(resolve("", { word_network: { family: [{ term: "garen" }] } }).sourceType, "structured");
  assert.deepEqual(resolve(markdown, null, "gar"), parse(markdown, "gar"));
  assert.equal(parse("#### 深度解析与避坑\n\n* 辨析：完整标题\n正文\n- 子项一\n- 子项二\n\n* 下一条\n正文").deepInsights.length, 2);
  const networkMarkdown = "#### 词汇网络\n**词族**\n* [duzen] (*v.*, *称呼*, *address*)\n* [Duzen] (*n.*, *称呼行为*, *addressing*)\n**同义词**\n* [Wort] (*n.*, *同义词*, *synonym*)\n* [Wort] (*n.*, *词汇*, *word*)\n* [Wort] (*n.*, *词汇*, *word*)";
  const variants = resolve(networkMarkdown, { ...core, word_network: { family: [{ term: "duzen", part_of_speech: "v.", chinese: "称呼", english: "address" }], synonyms: [], antonyms: [] } });
  assert.equal(variants.wordNetwork.family.length, 2);
  assert.equal(variants.wordNetwork.synonyms.length, 2);
  assert.equal(variants.wordNetwork.synonyms[0].chinese, "同义词");
  assert.deepEqual(parse("#### 语法详情\n| 特征 | 值 |\n| :--- | --- |\n| 形态特征 | 保留这一行 |\n| 比较级 | - |").grammarRows, [
    { key: "形态特征", value: "保留这一行" }, { key: "比较级", value: "-" },
  ]);
  console.log("structured analysis regression: passed");
} finally {
  await server.close();
}
