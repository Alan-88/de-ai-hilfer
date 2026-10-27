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

function parseDeepInsights(
  content: string,
  markdownToPlainText: (markdown: string) => string,
  renderMarkdownHtml: (markdown: string) => string
): StructuredAnalysisDeepInsight[] {
  if (!content) return [];
  const lines = content.split("\n");
  const insights: StructuredAnalysisDeepInsight[] = [];
  const hasBulletTitles = lines.some(isTopLevelInsightBullet);
  const hasBoldBulletTitles = lines.some((line) => /^[*-]\s+\*\*/.test(line));
  const bulletMarker = lines.find(isTopLevelInsightBullet)?.[0];

  let currentTitle = "";
  let currentLines: string[] = [];

  function flush() {
    if (!currentTitle) return;
    const body = currentLines.join("\n").trim();
    if (!body) return;
    insights.push(createDeepInsight(currentTitle, body, markdownToPlainText, renderMarkdownHtml));
  }

  for (const [index, line] of lines.entries()) {
    const bulletTitle = isTopLevelInsightBullet(line) && (hasBoldBulletTitles
      ? /^[*-]\s+\*\*/.test(line)
      : line[0] === bulletMarker && (!currentTitle || !lines[index - 1]?.trim()));
    if (bulletTitle || isInlineInsightHeading(line) || (!hasBulletTitles && isOrderedInsightHeading(line))) {
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
      const key = JSON.stringify(item);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

function isTopLevelInsightBullet(line: string): boolean {
  return /^(\*|-)\s+/.test(line);
}

function isInlineInsightHeading(line: string): boolean {
  return /^\*\*[^*]+\*\*[:：]?\s*$/.test(line);
}

function isOrderedInsightHeading(line: string): boolean {
  return /^\d+\.\s+/.test(line);
}

function normalizeInsightLead(line: string): string {
  return line.replace(/^(\*|-)\s+/, "").replace(/^\s*\d+\.\s+/, "").trim();
}
