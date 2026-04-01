import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { parseAnalysisMarkdown } from "../src/lib/analysis/structuredAnalysis";

interface StreamMeta {
  kind: string;
  model?: string;
  quality_mode?: string;
  source?: string;
  fallback?: boolean;
}

interface AnalyzeResponse {
  entry_id: number;
  query_text: string;
  analysis_markdown: string;
  source: string;
  model?: string;
  quality_mode?: string;
  follow_ups: unknown[];
}

interface UsageModuleSnapshot {
  title: string;
  explanation: string;
  exampleDe: string;
  exampleZh: string;
}

interface AttemptResult {
  attempt: number;
  ok: boolean;
  source?: string;
  model?: string;
  fallback?: boolean;
  rawMarkdown: string;
  usageModuleCount: number;
  reasons: string[];
  usageModules: UsageModuleSnapshot[];
}

interface CaseResult {
  query: string;
  ok: boolean;
  attempts: AttemptResult[];
  passCount: number;
  failCount: number;
}

function parseArgs(argv: string[]) {
  const args = new Map<string, string>();
  for (let i = 0; i < argv.length; i += 1) {
    const current = argv[i];
    if (!current.startsWith("--")) continue;
    const next = argv[i + 1];
    if (!next || next.startsWith("--")) {
      args.set(current, "true");
      continue;
    }
    args.set(current, next);
    i += 1;
  }
  return {
    baseUrl: args.get("--base-url") ?? "http://127.0.0.1:8000/api/v1",
    casesPath: resolve(args.get("--cases") ?? "../backend/eval/cases/pilot_20.txt"),
    qualityMode: args.get("--quality-mode") ?? "default",
    outputPath: args.get("--output") ? resolve(args.get("--output")!) : "",
    limit: args.get("--limit") ? Number(args.get("--limit")) : undefined,
    repeats: args.get("--repeats") ? Number(args.get("--repeats")) : 1,
  };
}

function readCases(content: string, limit?: number): string[] {
  const cases = content
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"));
  return limit ? cases.slice(0, limit) : cases;
}

async function runStreamAttempt(
  baseUrl: string,
  query: string,
  qualityMode: string,
  attempt: number
): Promise<AttemptResult> {
  const response = await fetch(`${baseUrl}/analyze/stream`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      query_text: query,
      quality_mode: qualityMode,
      force_refresh: true,
    }),
  });

  if (!response.ok || !response.body) {
    return {
      attempt,
      ok: false,
      rawMarkdown: "",
      usageModuleCount: 0,
      reasons: [`request_failed:${response.status}`],
      usageModules: [],
    };
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  let meta: StreamMeta | null = null;
  let complete: AnalyzeResponse | null = null;

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });

    while (buffer.includes("\n\n")) {
      const splitIndex = buffer.indexOf("\n\n");
      const rawEvent = buffer.slice(0, splitIndex).trim();
      buffer = buffer.slice(splitIndex + 2);
      if (!rawEvent) continue;

      const eventMatch = rawEvent.match(/^event:\s*(.+)$/m);
      const dataMatch = rawEvent.match(/^data:\s*(.+)$/m);
      const eventName = eventMatch?.[1]?.trim();
      const data = dataMatch?.[1]?.trim();
      if (!eventName || !data) continue;

      if (eventName === "meta") {
        meta = JSON.parse(data) as StreamMeta;
      }
      if (eventName === "complete") {
        complete = JSON.parse(data) as AnalyzeResponse;
      }
    }
  }

  if (!complete) {
    return {
      attempt,
      ok: false,
      source: meta?.source,
      model: meta?.model,
      fallback: meta?.fallback,
      rawMarkdown: "",
      usageModuleCount: 0,
      reasons: ["missing_complete_event"],
      usageModules: [],
    };
  }

  const structured = parseAnalysisMarkdown(complete.analysis_markdown, complete.query_text);
  const usageModules = structured.usageModules.map((item) => ({
    title: item.title,
    explanation: item.explanation,
    exampleDe: item.example?.de ?? "",
    exampleZh: item.example?.zh ?? "",
  }));

  const reasons: string[] = [];
  if (complete.source.includes("兜底")) reasons.push(`fallback_source:${complete.source}`);
  if (usageModules.length === 0) reasons.push("no_usage_modules");
  for (const [index, item] of usageModules.entries()) {
    if (!item.title) reasons.push(`module_${index + 1}_missing_title`);
    if (!item.explanation) reasons.push(`module_${index + 1}_missing_explanation`);
    if (!item.exampleDe) reasons.push(`module_${index + 1}_missing_example_de`);
    if (!item.exampleZh) reasons.push(`module_${index + 1}_missing_example_zh`);
  }

  return {
    attempt,
    ok: reasons.length === 0,
    source: complete.source,
    model: complete.model,
    fallback: meta?.fallback,
    rawMarkdown: complete.analysis_markdown,
    usageModuleCount: usageModules.length,
    reasons,
    usageModules,
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const caseContent = await readFile(args.casesPath, "utf-8");
  const cases = readCases(caseContent, args.limit);
  const results: CaseResult[] = [];

  for (const query of cases) {
    const attempts: AttemptResult[] = [];
    for (let attempt = 1; attempt <= args.repeats; attempt += 1) {
      const result = await runStreamAttempt(args.baseUrl, query, args.qualityMode, attempt);
      attempts.push(result);
      const status = result.ok ? "PASS" : "FAIL";
      const suffix = result.reasons.length > 0 ? ` ${result.reasons.join(", ")}` : "";
      console.log(
        `[${status}] ${query} attempt=${attempt}/${args.repeats} modules=${result.usageModuleCount}${suffix}`
      );
    }

    const passCount = attempts.filter((item) => item.ok).length;
    const failCount = attempts.length - passCount;
    results.push({
      query,
      ok: failCount === 0,
      attempts,
      passCount,
      failCount,
    });
  }

  const summary = {
    createdAt: new Date().toISOString(),
    baseUrl: args.baseUrl,
    casesPath: args.casesPath,
    qualityMode: args.qualityMode,
    repeats: args.repeats,
    total: results.length,
    passed: results.filter((item) => item.ok).length,
    failed: results.filter((item) => !item.ok).length,
    totalAttempts: results.reduce((sum, item) => sum + item.attempts.length, 0),
    failedAttempts: results.reduce((sum, item) => sum + item.failCount, 0),
    results,
  };

  if (args.outputPath) {
    await writeFile(args.outputPath, JSON.stringify(summary, null, 2), "utf-8");
    console.log(`report: ${args.outputPath}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
