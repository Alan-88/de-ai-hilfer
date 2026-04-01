export interface LearningExample {
  de: string;
  zh?: string;
}

export interface LearningQuiz {
  question: string;
  options: string[];
  answerIndex: number;
  explanation: string;
}

export interface LearningEnhancements {
  insights: string[];
  examples: LearningExample[];
  quiz: LearningQuiz | null;
}

export function buildLearningEnhancements(markdown: string, headword: string): LearningEnhancements {
  const plain = normalizeMarkdown(markdown);
  const insights = buildInsights(plain);
  const examples = buildExamples(markdown, headword);
  const quiz = buildQuiz(plain, headword);

  return { insights, examples, quiz };
}

function buildInsights(plain: string): string[] {
  const insights: string[] = [];
  const caseMatch = plain.match(/(支配|搭配格)[^。；\n]*?(第三格|第四格|第二格|第一格|Dativ|Akkusativ|Genitiv|Nominativ)/i);
  if (caseMatch) {
    insights.push(`语法焦点：${trimSentence(caseMatch[0])}`);
  }

  const formsMatch = plain.match(/(主要变位|关键形式|现在时|过去时|完成时)[^。；\n]{0,70}/i);
  if (formsMatch) {
    insights.push(`记忆重点：${trimSentence(formsMatch[0])}`);
  }

  const mistakeMatch = plain.match(/(常见错误|易错)[^。；\n]{0,90}/i);
  if (mistakeMatch) {
    insights.push(`避坑提示：${trimSentence(mistakeMatch[0])}`);
  }

  return insights.slice(0, 3);
}

function buildExamples(markdown: string, headword: string): LearningExample[] {
  const lines = markdown.split("\n").map((line) => line.trim());
  const examples: LearningExample[] = [];

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const deMatch = line.match(/^\d+\.\s*(.+)$/);
    if (!deMatch) continue;

    const de = stripInlineMarkdown(deMatch[1]);
    const nextLine = lines[i + 1] ?? "";
    const zh = nextLine ? stripInlineMarkdown(nextLine.replace(/^[\-*]\s*/, "")) : undefined;
    if (de) examples.push({ de, zh: zh || undefined });
  }

  if (examples.length > 0) {
    return examples.slice(0, 4);
  }

  const sentenceCandidates = normalizeMarkdown(markdown)
    .split(/[。！？\n]/)
    .map((part) => part.trim())
    .filter((part) => part.length >= 8)
    .filter((part) => /[A-Za-zÄÖÜäöüß]/.test(part))
    .filter((part) => part.toLowerCase().includes(headword.toLowerCase()));

  return sentenceCandidates.slice(0, 2).map((de) => ({ de }));
}

function buildQuiz(plain: string, headword: string): LearningQuiz | null {
  const caseInfo = detectCase(plain);
  if (caseInfo) {
    const options = ["第一格 (Nominativ)", "第二格 (Genitiv)", "第三格 (Dativ)", "第四格 (Akkusativ)"];
    const answerIndex = caseAnswerIndex(caseInfo);
    if (answerIndex >= 0) {
      return {
        question: `动词 ${headword} 常见搭配对象用哪个格？`,
        options,
        answerIndex,
        explanation: `解析：${headword} 的常见搭配提示中出现了 ${caseInfo}。`,
      };
    }
  }

  const genderInfo = detectGender(plain);
  if (genderInfo) {
    const options = ["der", "die", "das"];
    const answerIndex = options.indexOf(genderInfo);
    if (answerIndex >= 0) {
      return {
        question: `${headword} 的常见冠词是哪个？`,
        options,
        answerIndex,
        explanation: `解析：词条语法信息里给出的冠词是 ${genderInfo}。`,
      };
    }
  }

  return null;
}

function detectCase(plain: string): string | null {
  if (/第三格|Dativ/i.test(plain)) return "第三格 (Dativ)";
  if (/第四格|Akkusativ/i.test(plain)) return "第四格 (Akkusativ)";
  if (/第二格|Genitiv/i.test(plain)) return "第二格 (Genitiv)";
  if (/第一格|Nominativ/i.test(plain)) return "第一格 (Nominativ)";
  return null;
}

function caseAnswerIndex(caseInfo: string): number {
  if (caseInfo.includes("第一格")) return 0;
  if (caseInfo.includes("第二格")) return 1;
  if (caseInfo.includes("第三格")) return 2;
  if (caseInfo.includes("第四格")) return 3;
  return -1;
}

function detectGender(plain: string): string | null {
  const match = plain.match(/\b(der|die|das)\b/i);
  return match ? match[1].toLowerCase() : null;
}

function normalizeMarkdown(markdown: string): string {
  return markdown
    .replace(/\r/g, "")
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/[*_`>#|]/g, " ")
    .replace(/\[[^\]]*]\([^)]*\)/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function stripInlineMarkdown(value: string): string {
  return value
    .replace(/[*_`]/g, "")
    .replace(/\[[^\]]*]\([^)]*\)/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function trimSentence(value: string): string {
  return value
    .replace(/\s+/g, " ")
    .replace(/[;；。]+$/, "")
    .trim();
}
