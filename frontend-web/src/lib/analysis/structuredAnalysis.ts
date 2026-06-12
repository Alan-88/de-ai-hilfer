import { marked } from "marked";
import {
  parseUsageModules,
  type StructuredExample,
  type UsageModule,
} from "$lib/analysis/usageModules";
import {
  extractDeepInsights,
  flattenWordNetworkTerms,
  looksStructuredTooSparse,
  normalizeWordNetworkDocument,
  type StructuredAnalysisDeepInsight as DeepInsight,
  type StructuredAnalysisWordNetwork as WordNetwork,
  type StructuredAnalysisWordNetworkItem as WordNetworkItem,
} from "$lib/analysis/structuredAnalysisSupport";
import type { StructuredAnalysisDocument as BackendStructuredAnalysisDocument } from "$lib/types";

export interface StructuredMeaning {
  partOfSpeech: string;
  chinese: string;
  english: string;
}

export interface GrammarRow {
  key: string;
  value: string;
}

export interface StructuredAnalysis {
  headword: string;
  phonetic: string;
  meanings: StructuredMeaning[];
  usageModules: UsageModule[];
  collocations: string[];
  examples: StructuredExample[];
  grammarRows: GrammarRow[];
  grammarBranches: import("$lib/types").GrammarBranch[];
  wordNetwork: WordNetwork;
  deepInsights: DeepInsight[];
  rawMarkdown: string;
  sourceType: "structured" | "markdown";
}

type StructuredAnalysisPayload = Omit<StructuredAnalysis, "sourceType">;

export function resolveStructuredAnalysis(
  markdown: string,
  structured: BackendStructuredAnalysisDocument | null | undefined,
  fallbackHeadword = ""
): StructuredAnalysis {
  const normalizedStructured = normalizeStructuredAnalysisDocument(
    structured,
    markdown,
    fallbackHeadword
  );
  if (normalizedStructured) {
    return { ...normalizedStructured, sourceType: "structured" };
  }
  return { ...parseAnalysisMarkdown(markdown, fallbackHeadword), sourceType: "markdown" };
}

interface RawSection {
  level: number;
  title: string;
  content: string;
  index: number;
}

const SECTION_RE = /^####\s+(.+)$/gm;

export function parseAnalysisMarkdown(markdown: string, fallbackHeadword = ""): StructuredAnalysis {
  const normalized = (markdown || "").replace(/\r/g, "").trim();
  if (!normalized) return createEmptyAnalysis(fallbackHeadword);

  const allSections = getAllSections(normalized);
  const headword = extractHeadword(normalized, fallbackHeadword);
  const phonetic = extractPhonetic(normalized);

  const meanings = parseMeanings(getContentByKeywords(allSections, ["核心释义", "释义"]));
  const { usageModules, collocations, examples } = parseUsageModules(
    getContentByKeywords(allSections, ["应用与例句", "例句"]),
    (m) => m, 
    markdownToPlainText
  );
  const grammarRows = parseGrammarTable(getContentByKeywords(allSections, ["语法详情", "语法"]));
  const wordNetwork = parseWordNetwork(getContentByKeywords(allSections, ["词汇网络", "相关词"]));
  const deepInsights = extractDeepInsights(allSections, markdownToPlainText, renderMarkdownHtml);

  return {
    headword,
    phonetic,
    meanings,
    usageModules,
    collocations,
    examples,
    grammarRows,
    grammarBranches: [],
    wordNetwork,
    deepInsights,
    rawMarkdown: normalized,
    sourceType: "markdown",
  };
}

function normalizeStructuredAnalysisDocument(
  structured: BackendStructuredAnalysisDocument | null | undefined,
  markdown: string,
  fallbackHeadword: string
): StructuredAnalysisPayload | null {
  if (!structured) return null;

  const rawMarkdown = (markdown || "").replace(/\r/g, "").trim();
  const wordNetwork = normalizeWordNetworkDocument(structured);
  const family = flattenWordNetworkTerms(wordNetwork.family, markdownToPlainText);
  const synonyms = flattenWordNetworkTerms(wordNetwork.synonyms, markdownToPlainText);
  const antonyms = flattenWordNetworkTerms(wordNetwork.antonyms, markdownToPlainText);

  const normalized = {
    headword: (structured.headword || fallbackHeadword || "").trim(),
    phonetic: (structured.phonetic || "").trim(),
    meanings: (structured.meanings || [])
      .map((meaning) => ({
        partOfSpeech: (meaning.part_of_speech || "").trim(),
        chinese: (meaning.chinese || "").trim(),
        english: (meaning.english || "").trim(),
      }))
      .filter((meaning) => meaning.chinese || meaning.english),
    usageModules: (structured.usage_modules || [])
      .map((module) => ({
        title: (module.title || "").trim(),
        explanation: (module.explanation || "").trim(),
        example:
          module.example_de?.trim() || module.example_zh?.trim()
            ? {
                de: (module.example_de || "").trim(),
                zh: (module.example_zh || "").trim(),
              }
            : null,
      }))
      .filter((module) => module.title || module.explanation || module.example?.de || module.example?.zh),
    collocations: dedupeStrings(structured.collocations || []),
    examples: (structured.examples || [])
      .map((example) => ({
        de: (example.de || "").trim(),
        zh: (example.zh || "").trim(),
      }))
      .filter((example) => example.de || example.zh),
    grammarRows: (structured.grammar_rows || [])
      .map((row) => ({
        key: (row.key || "").trim(),
        value: (row.value || "").trim(),
      }))
      .filter((row) => row.key && row.value),
    grammarBranches: structured.grammar_branches || [],
    wordNetwork,
    deepInsights: (structured.deep_insights || [])
      .map((insight) => {
        const title = markdownToPlainText((insight.title || "").trim()) || "分析片段";
        const body = (insight.content_markdown || "").trim();
        return body
          ? {
              title,
              plain: markdownToPlainText(body),
              html: renderMarkdownHtml(body),
            }
          : null;
      })
      .filter((insight): insight is DeepInsight => Boolean(insight)),
    rawMarkdown,
  } satisfies StructuredAnalysisPayload;

  if (looksStructuredTooSparse(normalized, rawMarkdown)) {
    return null;
  }

  const isEmpty =
    !normalized.phonetic &&
    normalized.meanings.length === 0 &&
    normalized.usageModules.length === 0 &&
    normalized.collocations.length === 0 &&
    normalized.examples.length === 0 &&
    normalized.grammarRows.length === 0 &&
    normalized.deepInsights.length === 0;

  return isEmpty ? null : normalized;
}

function getAllSections(markdown: string): RawSection[] {
  const sections: RawSection[] = [];
  const matches = [...markdown.matchAll(SECTION_RE)];

  for (let i = 0; i < matches.length; i++) {
    const match = matches[i];
    const level = 4; // 严格回归 ####
    const title = markdownToPlainText(match[1]);
    const start = match.index! + match[0].length;
    const end = matches[i + 1] ? matches[i + 1].index : markdown.length;
    sections.push({
      level,
      title: normalizeSectionTitle(title),
      content: markdown.slice(start, end).trim(),
      index: match.index!
    });
  }
  return sections;
}

function normalizeSectionTitle(value: string): string {
  const stripped = value.replace(/\s*\(.*?\)\s*/g, " ").trim();
  if (stripped.includes("核心释义")) return "核心释义";
  if (stripped.includes("应用与例句")) return "应用与例句";
  if (stripped.includes("语法详情")) return "语法详情";
  if (stripped.includes("词汇网络")) return "词汇网络";
  if (stripped.includes("深度解析")) return "深度解析";
  return stripped;
}

function getContentByKeywords(sections: RawSection[], keywords: string[]): string {
  const match = sections.find(s => keywords.some(k => s.title.includes(k)));
  return match ? match.content : "";
}

function extractHeadword(markdown: string, fallback: string): string {
  const match = markdown.match(/^###\s+\**([^*\n]+?)\**\s*$/m);
  return match ? match[1].trim() : fallback;
}

function extractPhonetic(markdown: string): string {
  const match = markdown.match(/^\/[^/\n]+\/$/m);
  return match ? match[0].trim() : "";
}

function parseMeanings(content: string): StructuredMeaning[] {
  if (!content) return [];
  return content.split("\n")
    .map(l => l.trim())
    .filter(l => l.startsWith("*"))
    .map(line => {
      const clean = line.replace(/^\*\s+/, "");
      const bolds = [...clean.matchAll(/\*\*([^*]+)\*\*/g)].map(m => m[1].trim());
      const italics = clean.match(/\*([^*]+)\*\s*$/);
      return {
        partOfSpeech: bolds[0] || "",
        chinese: bolds[1] || markdownToPlainText(clean),
        english: italics ? italics[1].trim() : ""
      };
    }).filter(m => m.chinese);
}

function parseGrammarTable(content: string): GrammarRow[] {
  if (!content) return [];
  return content.split("\n")
    .filter(l => l.includes("|") && !l.includes("---"))
    .map(line => {
      const cells = line.split("|").map(c => markdownToPlainText(c.trim())).filter(Boolean);
      return { key: cells[0] || "", value: cells[1] || "" };
    })
    .filter(row => row.key && row.value && !row.key.includes("特征"));
}

function parseWordNetwork(content: string): WordNetwork {
  const res: WordNetwork = { family: [], synonyms: [], antonyms: [] };
  if (!content) return res;
  let current: keyof typeof res | null = null;
  for (const line of content.split("\n")) {
    const p = markdownToPlainText(line);
    if (!p) continue;
    if (p.includes("词族")) { current = "family"; continue; }
    if (p.includes("同义词")) { current = "synonyms"; continue; }
    if (p.includes("反义词")) { current = "antonyms"; continue; }
    if (current && (line.trim().startsWith("*") || line.includes("    *"))) {
      res[current].push({
        term: p,
        partOfSpeech: "",
        chinese: "",
        english: "",
        note: "",
      });
    }
  }
  return res;
}

function createEmptyAnalysis(headword: string): StructuredAnalysis {
  return {
    headword, phonetic: "", meanings: [], usageModules: [], collocations: [],
    examples: [], grammarRows: [], grammarBranches: [], wordNetwork: { family: [], synonyms: [], antonyms: [] },
    deepInsights: [], rawMarkdown: "",
    sourceType: "markdown"
  };
}

function dedupeStrings(values: string[]): string[] {
  const seen = new Set<string>();
  return values
    .map((value) => markdownToPlainText(value))
    .filter((value) => !!value)
    .filter((value) => {
      const key = value.toLowerCase();
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

export function markdownToPlainText(markdown: string): string {
  return markdown
    .replace(/\r/g, "")
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/\[(.*?)\]\((.*?)\)/g, "$1")
    .replace(/[*_`>#|]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function renderMarkdownHtml(markdown: string): string {
  return marked.parse(markdown) as string;
}
