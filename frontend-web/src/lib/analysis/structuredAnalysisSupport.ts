import type { StructuredAnalysisDocument as BackendStructuredAnalysisDocument } from "$lib/types";

export interface StructuredAnalysisDeepInsight {
  title: string;
  plain: string;
  html: string;
}

export interface StructuredAnalysisWordNetworkItem {
  term: string;
  partOfSpeech: string;
  chinese: string;
  english: string;
  note: string;
}

export interface StructuredAnalysisWordNetwork {
  family: StructuredAnalysisWordNetworkItem[];
  synonyms: StructuredAnalysisWordNetworkItem[];
  antonyms: StructuredAnalysisWordNetworkItem[];
}

export interface StructuredAnalysisSectionLike {
  title: string;
  content: string;
}

export interface StructuredAnalysisSparseCandidate {
  usageModules: Array<{
    title?: string;
    explanation?: string;
    example?: {
      de?: string;
      zh?: string;
    } | null;
  }>;
  grammarRows: Array<unknown>;
  wordNetwork: StructuredAnalysisWordNetwork;
  deepInsights: StructuredAnalysisDeepInsight[];
}

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

export function extractDeepInsights(
  sections: StructuredAnalysisSectionLike[],
  markdownToPlainText: (markdown: string) => string,
  renderMarkdownHtml: (markdown: string) => string
): StructuredAnalysisDeepInsight[] {
  const insights: StructuredAnalysisDeepInsight[] = [];
  const deepSections = sections.filter((section) =>
    DEEP_INSIGHT_TITLES.some((title) => section.title.includes(title))
  );

  for (const section of deepSections) {
    if (!section.content.trim()) continue;

    if (section.title.includes("深度解析") || section.title.includes("避坑")) {
      const parsed = parseDeepInsights(section.content, markdownToPlainText, renderMarkdownHtml);
      if (parsed.length > 0) {
        insights.push(...parsed);
        continue;
      }
    }

    insights.push(createDeepInsight(section.title, section.content, markdownToPlainText, renderMarkdownHtml));
  }

  return dedupeDeepInsights(insights);
}

export function normalizeWordNetworkDocument(
  structured: BackendStructuredAnalysisDocument
): StructuredAnalysisWordNetwork {
  const network = structured.word_network ?? {
    family: [],
    synonyms: [],
    antonyms: [],
  };

  return {
    family: dedupeWordNetworkItems(network.family || []),
    synonyms: dedupeWordNetworkItems(network.synonyms || []),
    antonyms: dedupeWordNetworkItems(network.antonyms || []),
  };
}

export function flattenWordNetworkTerms(
  items: StructuredAnalysisWordNetworkItem[],
  markdownToPlainText: (markdown: string) => string
): string[] {
  return dedupeStrings(
    items.map((item) => {
      const extra = [item.partOfSpeech, item.chinese || item.english || item.note]
        .filter(Boolean)
        .join(" · ");
      return extra ? `${item.term} · ${extra}` : item.term;
    }),
    markdownToPlainText
  );
}

export function looksStructuredTooSparse(
  structured: StructuredAnalysisSparseCandidate,
  markdown: string
): boolean {
  if (!markdown.trim()) return false;

  const hasUsageSection = markdown.includes("#### 应用与例句");
  const hasGrammarSection = markdown.includes("#### 语法详情");
  const hasDeepInsightSection =
    markdown.includes("#### 深度解析") ||
    markdown.includes("#### 深度解析与避坑") ||
    markdown.includes("#### 避坑");
  const hasWordNetworkSection = markdown.includes("#### 词汇网络");

  if (hasUsageSection) {
    const completeUsageModules = structured.usageModules.filter(
      (module) =>
        module.title &&
        module.explanation &&
        module.example?.de &&
        module.example?.zh
    ).length;
    const markdownModuleCount = countUsageLikeBlocks(markdown);
    if (markdownModuleCount > 0 && completeUsageModules === 0) {
      return true;
    }
  }

  if (hasGrammarSection && markdown.includes("| :--- |") && structured.grammarRows.length === 0) {
    return true;
  }

  if (hasDeepInsightSection && structured.deepInsights.length === 0) {
    return true;
  }

  if (
    markdown.includes("词源") &&
    !structured.deepInsights.some((item) => item.title.includes("词源"))
  ) {
    return true;
  }

  if (
    hasWordNetworkSection &&
    structured.wordNetwork.family.length === 0 &&
    structured.wordNetwork.synonyms.length === 0 &&
    structured.wordNetwork.antonyms.length === 0
  ) {
    return true;
  }

  return false;
}

function parseDeepInsights(
  content: string,
  markdownToPlainText: (markdown: string) => string,
  renderMarkdownHtml: (markdown: string) => string
): StructuredAnalysisDeepInsight[] {
  if (!content) return [];
  const lines = content.split("\n");
  const insights: StructuredAnalysisDeepInsight[] = [];

  let currentTitle = "";
  let currentLines: string[] = [];

  function flush() {
    if (!currentTitle) return;
    const body = currentLines.join("\n").trim();
    if (!body) return;
    insights.push(createDeepInsight(currentTitle, body, markdownToPlainText, renderMarkdownHtml));
  }

  for (const line of lines) {
    if (isTopLevelInsightBullet(line) || isInlineInsightHeading(line) || isOrderedInsightHeading(line)) {
      flush();
      const rawTitle = normalizeInsightLead(line);
      const { title, rest } = splitInsightLead(rawTitle, markdownToPlainText);
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

function createDeepInsight(
  title: string,
  body: string,
  markdownToPlainText: (markdown: string) => string,
  renderMarkdownHtml: (markdown: string) => string
): StructuredAnalysisDeepInsight {
  const normalizedTitle = markdownToPlainText(title) || "分析片段";
  const normalizedBody = body.trim();
  return {
    title: normalizedTitle,
    plain: markdownToPlainText(normalizedBody),
    html: renderMarkdownHtml(normalizedBody),
  };
}

function splitInsightLead(
  rawTitle: string,
  markdownToPlainText: (markdown: string) => string
): { title: string; rest: string } {
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

function dedupeDeepInsights(insights: StructuredAnalysisDeepInsight[]): StructuredAnalysisDeepInsight[] {
  const seen = new Set<string>();
  return insights.filter((insight) => {
    const key = `${insight.title}::${insight.plain}`;
    if (!insight.plain || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function dedupeWordNetworkItems(items: Array<{
  term?: string;
  part_of_speech?: string;
  chinese?: string;
  english?: string;
  note?: string;
}>): StructuredAnalysisWordNetworkItem[] {
  const seen = new Set<string>();
  return items
    .map((item) => ({
      term: (item.term || "").trim(),
      partOfSpeech: (item.part_of_speech || "").trim(),
      chinese: (item.chinese || "").trim(),
      english: (item.english || "").trim(),
      note: (item.note || "").trim(),
    }))
    .filter((item) => item.term)
    .filter((item) => {
      const key = item.term.toLowerCase();
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

function dedupeStrings(
  values: string[],
  markdownToPlainText: (markdown: string) => string
): string[] {
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

function countUsageLikeBlocks(markdown: string): number {
  return markdown
    .split("\n\n")
    .filter((block) => {
      const lines = block
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean);
      return (
        lines.length >= 3 &&
        !lines[0].startsWith("####") &&
        !lines[0].startsWith("|") &&
        /[A-Za-zÄÖÜäöüß]/.test(lines[1]) &&
        (lines[2].startsWith("（") || lines[2].startsWith("("))
      );
    })
    .length;
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
