import { marked } from "marked";
import {
  parseUsageModules,
  type StructuredExample,
  type UsageModule,
} from "$lib/analysis/usageModules";

export interface StructuredMeaning {
  partOfSpeech: string;
  chinese: string;
  english: string;
}

export interface GrammarRow {
  key: string;
  value: string;
}

export interface DeepInsight {
  title: string;
  plain: string;
  html: string;
}

export interface StructuredAnalysis {
  headword: string;
  phonetic: string;
  meanings: StructuredMeaning[];
  usageModules: UsageModule[];
  collocations: string[];
  examples: StructuredExample[];
  grammarRows: GrammarRow[];
  family: string[];
  synonyms: string[];
  antonyms: string[];
  deepInsights: DeepInsight[];
  rawMarkdown: string;
}

interface RawSection {
  level: number;
  title: string;
  content: string;
  index: number;
}

const SECTION_RE = /^####\s+(.+)$/gm;
const DEEP_INSIGHT_TITLES = [
  "深度解析",
  "避坑",
  "多维解释",
  "词源",
  "常见错误",
  "易错",
  "辨析",
  "使用提醒",
];

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
  const { family, synonyms, antonyms } = parseWordNetwork(getContentByKeywords(allSections, ["词汇网络", "相关词"]));
  
  const deepInsights = extractDeepInsights(allSections);

  return {
    headword,
    phonetic,
    meanings,
    usageModules,
    collocations,
    examples,
    grammarRows,
    family,
    synonyms,
    antonyms,
    deepInsights,
    rawMarkdown: normalized,
  };
}

function extractDeepInsights(sections: RawSection[]): DeepInsight[] {
  const insights: DeepInsight[] = [];
  const deepSections = sections.filter((section) =>
    DEEP_INSIGHT_TITLES.some((title) => section.title.includes(title))
  );

  for (const section of deepSections) {
    if (!section.content.trim()) continue;

    if (section.title.includes("深度解析") || section.title.includes("避坑")) {
      const parsed = parseDeepInsights(section.content);
      if (parsed.length > 0) {
        insights.push(...parsed);
        continue;
      }
    }

    insights.push(createDeepInsight(section.title, section.content));
  }

  return dedupeDeepInsights(insights);
}

function parseDeepInsights(content: string): DeepInsight[] {
  if (!content) return [];
  const lines = content.split("\n");
  const insights: DeepInsight[] = [];

  let currentTitle = "";
  let currentLines: string[] = [];

  function flush() {
    if (!currentTitle) return;
    const body = currentLines.join("\n").trim();
    if (!body) return;
    insights.push(createDeepInsight(currentTitle, body));
  }

  for (const line of lines) {
    if (isTopLevelInsightBullet(line) || isInlineInsightHeading(line) || isOrderedInsightHeading(line)) {
      flush();
      const rawTitle = normalizeInsightLead(line);
      const { title, rest } = splitInsightLead(rawTitle);
      currentTitle = title;
      currentLines = rest ? [rest] : [];
    } else if (currentTitle) {
      const cleanLine = line.replace(/^ {1,4}/, "");
      currentLines.push(cleanLine);
    }
  }
  flush();
  return insights;
}

function isTopLevelInsightBullet(line: string): boolean {
  return /^(\*|-)\s+/.test(line);
}

function isInlineInsightHeading(line: string): boolean {
  return /^\*\*[^*]+\*\*[:：]?\s*$/.test(line.trim());
}

function isOrderedInsightHeading(line: string): boolean {
  return /^\s*\d+\.\s+/.test(line);
}

function normalizeInsightLead(line: string): string {
  return line.replace(/^(\*|-)\s+/, "").replace(/^\s*\d+\.\s+/, "").trim();
}

function splitInsightLead(rawTitle: string): { title: string; rest: string } {
  const boldOnly = rawTitle.match(/^\*\*([^*]+)\*\*[:：]?(.*)$/);
  if (boldOnly) {
    return {
      title: markdownToPlainText(boldOnly[1]),
      rest: boldOnly[2].trim(),
    };
  }

  const colonIndex = rawTitle.search(/[:：]/);
  if (colonIndex >= 0) {
    return {
      title: markdownToPlainText(rawTitle.slice(0, colonIndex)),
      rest: rawTitle.slice(colonIndex + 1).trim(),
    };
  }

  return {
    title: markdownToPlainText(rawTitle),
    rest: "",
  };
}

function createDeepInsight(title: string, body: string): DeepInsight {
  const normalizedTitle = markdownToPlainText(title) || "分析片段";
  const normalizedBody = body.trim();
  return {
    title: normalizedTitle,
    plain: markdownToPlainText(normalizedBody),
    html: renderMarkdownHtml(normalizedBody),
  };
}

function dedupeDeepInsights(insights: DeepInsight[]): DeepInsight[] {
  const seen = new Set<string>();
  return insights.filter((insight) => {
    const key = `${insight.title}::${insight.plain}`;
    if (!insight.plain || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
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

function parseWordNetwork(content: string): { family: string[], synonyms: string[], antonyms: string[] } {
  const res = { family: [], synonyms: [], antonyms: [] };
  if (!content) return res;
  let current: keyof typeof res | null = null;
  for (const line of content.split("\n")) {
    const p = markdownToPlainText(line);
    if (!p) continue;
    if (p.includes("词族")) { current = "family"; continue; }
    if (p.includes("同义词")) { current = "synonyms"; continue; }
    if (p.includes("反义词")) { current = "antonyms"; continue; }
    if (current && (line.trim().startsWith("*") || line.includes("    *"))) {
      // @ts-ignore
      res[current].push(p);
    }
  }
  return res;
}

function createEmptyAnalysis(headword: string): StructuredAnalysis {
  return {
    headword, phonetic: "", meanings: [], usageModules: [], collocations: [],
    examples: [], grammarRows: [], family: [], synonyms: [], antonyms: [],
    deepInsights: [], rawMarkdown: ""
  };
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
