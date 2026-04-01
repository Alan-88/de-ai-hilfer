export interface StructuredExample {
  de: string;
  zh: string;
}

export interface UsageModule {
  title: string;
  explanation: string;
  example: StructuredExample | null;
}

export interface ParsedUsageModules {
  usageModules: UsageModule[];
  collocations: string[];
  examples: StructuredExample[];
}

const TOP_LIST_ITEM_RE = /^\*\s+/;
const NESTED_LIST_ITEM_RE = /^\s{2,}\*\s+/;
const ORDERED_ITEM_RE = /^\s*\d+\.\s+/;

export function parseUsageModules(
  section: string,
  normalizeMarkdownForRender: (markdown: string) => string,
  markdownToPlainText: (markdown: string) => string
): ParsedUsageModules {
  const normalizedSection = normalizeMarkdownForRender(section);
  const blockParsed = parseUsageModulesFromBlocks(normalizedSection, markdownToPlainText);
  if (blockParsed.usageModules.length > 0) {
    return blockParsed;
  }

  const lines = normalizedSection.split("\n");
  const usageModules: UsageModule[] = [];
  const collocations: string[] = [];
  const examples: StructuredExample[] = [];
  let currentMode: "collocation" | "example" | null = null;

  let currentModule: UsageModule | null = null;

  function flushModule() {
    if (!currentModule?.title) return;
    usageModules.push(currentModule);
    collocations.push(currentModule.title);
    if (currentModule.example?.de) {
      examples.push(currentModule.example);
    }
    currentModule = null;
  }

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index].trimEnd();
    const plain = markdownToPlainText(line);
    if (!plain) continue;

    if (TOP_LIST_ITEM_RE.test(line) && !plain.includes("固定搭配") && !plain.includes("例句")) {
      flushModule();
      currentMode = null;
      currentModule = {
        title: plain,
        explanation: "",
        example: null,
      };
      continue;
    }

    if (currentModule && NESTED_LIST_ITEM_RE.test(line)) {
      const body = line.replace(NESTED_LIST_ITEM_RE, "").trim();
      const normalizedBody = markdownToPlainText(body);
      if (normalizedBody.startsWith("用法解析")) {
        currentModule.explanation = normalizedBody.replace(/^用法解析[:：]?\s*/, "").trim();
        continue;
      }
      if (normalizedBody.startsWith("场景例句")) {
        const de = normalizedBody.replace(/^场景例句[:：]?\s*/, "").trim();
        currentModule.example = { de, zh: currentModule.example?.zh ?? "" };
        continue;
      }
      if (normalizedBody.startsWith("例句翻译")) {
        const zh = normalizedBody.replace(/^例句翻译[:：]?\s*/, "").trim();
        currentModule.example = { de: currentModule.example?.de ?? "", zh };
        continue;
      }
    }

    if (plain.includes("固定搭配")) {
      flushModule();
      currentMode = "collocation";
      continue;
    }

    if (plain.includes("例句")) {
      flushModule();
      currentMode = "example";
      continue;
    }

    if (currentMode === "collocation" && NESTED_LIST_ITEM_RE.test(line)) {
      collocations.push(markdownToPlainText(line));
      continue;
    }

    if (currentMode === "example" && ORDERED_ITEM_RE.test(line)) {
      const de = markdownToPlainText(line.replace(ORDERED_ITEM_RE, ""));
      const nextLine = lines[index + 1]?.trim() ?? "";
      const zh = nextLine && !ORDERED_ITEM_RE.test(nextLine) ? markdownToPlainText(nextLine) : "";
      examples.push({ de, zh });
      continue;
    }
  }

  flushModule();

  if (usageModules.length === 0 && examples.length === 0) {
    const plainSentences = markdownToPlainText(section)
      .split(/[。！？]/)
      .map((item) => item.trim())
      .filter((item) => item.length > 12);
    if (plainSentences[0]) {
      examples.push({ de: plainSentences[0], zh: plainSentences[1] ?? "" });
    }
  }

  return { usageModules, collocations, examples };
}

function parseUsageModulesFromBlocks(
  section: string,
  markdownToPlainText: (markdown: string) => string
): ParsedUsageModules {
  const usageModules: UsageModule[] = [];
  const collocations: string[] = [];
  const examples: StructuredExample[] = [];
  const blocks = section
    .split(/\n\s*\n/)
    .map((block) => block.trim())
    .filter(Boolean);

  for (const block of blocks) {
    const lines = block
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);

    if (lines.length < 2) continue;
    if (lines.some((line) => /^(固定搭配|例句)\b/.test(markdownToPlainText(line)))) continue;

    const parsed = parseUsageModuleBlock(lines, markdownToPlainText);
    if (!parsed) continue;

    usageModules.push(parsed);
    collocations.push(parsed.title);
    if (parsed.example?.de) {
      examples.push(parsed.example);
    }
  }

  return { usageModules, collocations, examples };
}

function parseUsageModuleBlock(
  lines: string[],
  markdownToPlainText: (markdown: string) => string
): UsageModule | null {
  let title = "";
  let explanation = "";
  let exampleDe = "";
  let exampleZh = "";

  const firstLinePlain = markdownToPlainText(stripListPrefix(lines[0])).trim();
  const inlineSplit = splitUsageHeadline(firstLinePlain);

  if (inlineSplit) {
    title = inlineSplit.title;
    explanation = inlineSplit.explanation;
  }

  for (const line of lines) {
    const plain = markdownToPlainText(stripListPrefix(line)).trim();
    if (!plain) continue;

    if (!title && plain === firstLinePlain && !isLabeledUsageLine(plain) && !looksLikeTranslationLine(plain)) {
      title = cleanupUsageTitle(plain);
      continue;
    }

    if (inlineSplit && plain === firstLinePlain) {
      continue;
    }

    if (plain.startsWith("用法解析")) {
      explanation = plain.replace(/^用法解析[:：]?\s*/, "").trim();
      continue;
    }

    if (plain.startsWith("场景例句")) {
      exampleDe = plain.replace(/^场景例句[:：]?\s*/, "").trim();
      continue;
    }

    if (plain.startsWith("例句翻译")) {
      exampleZh = plain.replace(/^例句翻译[:：]?\s*/, "").trim();
      continue;
    }

    if (!explanation && looksLikeExplanationLine(plain)) {
      explanation = plain;
      continue;
    }

    if (!exampleZh && looksLikeTranslationLine(plain)) {
      exampleZh = plain.replace(/^[（(]\s*/, "").replace(/\s*[）)]$/, "").trim();
      continue;
    }

    if (!exampleDe && looksLikeGermanSentence(plain)) {
      const inlineExample = splitInlineExampleAndTranslation(plain);
      exampleDe = inlineExample.de;
      exampleZh = exampleZh || inlineExample.zh;
    }
  }

  if (!title) return null;
  if (!explanation && !exampleDe) return null;

  return {
    title,
    explanation,
    example: exampleDe ? { de: exampleDe, zh: exampleZh } : null,
  };
}

function splitUsageHeadline(line: string): { title: string; explanation: string } | null {
  const labeledMatch = line.match(/^(.+?)\s+用法解析[:：]\s*(.+)$/);
  if (labeledMatch) {
    const title = cleanupUsageTitle(labeledMatch[1]);
    const explanation = labeledMatch[2]?.trim() ?? "";
    if (!title || !explanation) return null;
    return { title, explanation };
  }

  const slashMatch = line.match(/^(.+?)\s*\/\s*(.+)$/);
  if (slashMatch) {
    const right = slashMatch[2]?.trim() ?? "";
    if (/[\u4e00-\u9fff]/.test(right)) {
      const title = cleanupUsageTitle(slashMatch[1]);
      const explanation = right.replace(/^(句型结构|用法|用法解析)[:：]?\s*/, "").trim();
      if (!title || !explanation) return null;
      return { title, explanation };
    }
  }

  const match = line.match(/^(.+?)(?:[:：])\s+(.+)$/);
  if (!match) return null;

  const title = cleanupUsageTitle(match[1]);
  const explanation = match[2]?.trim() ?? "";
  if (!title || !explanation) return null;
  return { title, explanation };
}

function cleanupUsageTitle(value: string): string {
  return value
    .replace(/^\**\s*/, "")
    .replace(/\s*\**$/, "")
    .replace(/^\[(.+)\]$/, "$1")
    .replace(/\*\*/g, "")
    .trim();
}

function stripListPrefix(line: string): string {
  return line.replace(TOP_LIST_ITEM_RE, "").replace(NESTED_LIST_ITEM_RE, "").trim();
}

function isLabeledUsageLine(line: string): boolean {
  return /^(用法解析|场景例句|例句翻译)[:：]?/.test(line);
}

function looksLikeTranslationLine(line: string): boolean {
  return /^[（(].+[）)]$/.test(line);
}

function looksLikeExplanationLine(line: string): boolean {
  if (!line) return false;
  if (isLabeledUsageLine(line) || looksLikeTranslationLine(line)) return false;
  return /[\u4e00-\u9fff]/.test(line);
}

function looksLikeGermanSentence(line: string): boolean {
  if (!line) return false;
  if (/^(用法解析|场景例句|例句翻译)[:：]?/.test(line)) return false;
  if (/[\u4e00-\u9fff]/.test(line)) return false;
  return /[A-Za-zÄÖÜäöüß]/.test(line);
}

function splitInlineExampleAndTranslation(line: string): StructuredExample {
  const match = line.match(/^(.*?)(?:\s+[（(]([^()]+)[）)])$/);
  if (!match) {
    return { de: line, zh: "" };
  }

  return {
    de: match[1].trim(),
    zh: match[2].trim(),
  };
}
