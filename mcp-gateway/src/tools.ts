import { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";

import { DeaiApi, errorMessage, runMany } from "./backend.js";
import { DEAI_GUIDES, DEAI_GUIDE_TOPICS, LEARNING_GUIDE, SERVER_INSTRUCTIONS } from "./guide.js";

const positiveIds = z.array(z.number().int().positive()).min(1).max(100);
const readOnly = {
  readOnlyHint: true,
  destructiveHint: false,
  idempotentHint: true,
  openWorldHint: false,
} as const;
const writesData = {
  readOnlyHint: false,
  destructiveHint: false,
  idempotentHint: false,
  openWorldHint: true,
} as const;
const destructiveWrite = {
  readOnlyHint: false,
  destructiveHint: true,
  idempotentHint: false,
  openWorldHint: false,
} as const;

export function buildMcpServer(api: DeaiApi): McpServer {
  const server = new McpServer(
    { name: "de-ai-hilfer", version: "0.2.0" },
    { instructions: SERVER_INSTRUCTIONS },
  );

  server.registerTool(
    "deai_guide",
    {
      title: "Understand De-AI-Hilfer",
      description:
        "Read versioned guidance for De-AI-Hilfer's data model and article-first German learning workflows. " +
        "Use overview before the first multi-step task, then request the specific article, vocabulary, scheduler, diagnostic or correction topic as needed.",
      inputSchema: z.object({
        topic: z.enum(DEAI_GUIDE_TOPICS),
      }),
      annotations: readOnly,
    },
    async ({ topic }) => handle(() => DEAI_GUIDES[topic]),
  );

  server.registerTool(
    "lexeme_query",
    {
      title: "Query German lexemes",
      description:
        "Read-only vocabulary lookup. action=resolve requires surfaces and returns dictionary candidates. " +
        "action=inspect requires lexeme_ids and returns the knowledge entry and learning state. " +
        "Keep source sentences in the conversation; do not send them through this tool.",
      inputSchema: z.object({
        action: z.enum(["resolve", "inspect"]),
        surfaces: z.array(z.string().trim().min(1)).min(1).max(100).optional(),
        lexeme_ids: positiveIds.optional(),
        limit: z.number().int().min(1).max(20).optional(),
      }),
      annotations: readOnly,
    },
    async ({ action, surfaces, lexeme_ids: lexemeIds, limit }) =>
      handle(async () => {
        if (action === "resolve") {
          requireValues(surfaces, "surfaces");
          return runMany(surfaces, (surface) =>
            api.get(`/agent/lexemes/resolve?q=${encodeURIComponent(surface)}&limit=${limit ?? 8}`),
          );
        }
        requireValues(lexemeIds, "lexeme_ids");
        return runMany(lexemeIds, (lexemeId) => api.get(`/agent/lexemes/${lexemeId}`));
      }),
  );

  server.registerTool(
    "lexeme_initialize",
    {
      title: "Initialize vocabulary entries",
      description:
        "Initialize user-confirmed missing lexemes through the existing AI analysis pipeline. " +
        "This writes knowledge entries and can take several minutes. Do not call it until the user has explicitly confirmed the selected headwords. " +
        "provider_name and model_id must be supplied together or both omitted.",
      inputSchema: z.object({
        lexeme_ids: positiveIds,
        provider_name: z.string().trim().min(1).optional(),
        model_id: z.string().trim().min(1).optional(),
      }),
      annotations: writesData,
    },
    async ({ lexeme_ids: lexemeIds, provider_name: providerName, model_id: modelId }) =>
      handle(async () => {
        if (Boolean(providerName) !== Boolean(modelId)) {
          throw new Error("provider_name and model_id must be supplied together");
        }
        const body = providerName
          ? { model_override: { provider_name: providerName, model_id: modelId } }
          : {};
        return runMany(lexemeIds, async (lexemeId) => compactInitialization(await api.initializeLexeme(lexemeId, body)));
      }),
  );

  server.registerTool(
    "learning_query",
    {
      title: "Inspect learning state",
      description:
        "Read-only learning operations. Supported actions: list, status, logs, plan, explain. " +
        "status requires lexeme_ids. logs accepts date, entry_id, session_id and limit. " +
        "plan reconstructs today's safe plan but not the exact cursor of an open browser session.",
      inputSchema: z.object({
        action: z.enum(["list", "status", "logs", "plan", "explain"]),
        lexeme_ids: positiveIds.optional(),
        date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).optional(),
        entry_id: z.number().int().positive().optional(),
        session_id: z.string().trim().min(1).optional(),
        limit: z.number().int().min(1).max(500).optional(),
      }),
      annotations: readOnly,
    },
    async ({ action, lexeme_ids: lexemeIds, date, entry_id: entryId, session_id: sessionId, limit }) =>
      handle(async () => {
        switch (action) {
          case "list":
            return api.get("/agent/learning");
          case "status":
            requireValues(lexemeIds, "lexeme_ids");
            return runMany(lexemeIds, (lexemeId) => api.get(`/agent/lexemes/${lexemeId}`));
          case "logs": {
            const query = new URLSearchParams({ limit: String(limit ?? 100) });
            if (date) query.set("date", date);
            if (entryId) query.set("entry_id", String(entryId));
            if (sessionId) query.set("session_id", sessionId);
            return api.get(`/agent/learning/logs?${query.toString()}`);
          }
          case "plan":
            return api.get("/agent/learning/plan");
          case "explain":
            return { version: "V3", guide: LEARNING_GUIDE };
        }
      }),
  );

  server.registerTool(
    "learning_membership",
    {
      title: "Manage the learning plan",
      description:
        "Add, remove or reset confirmed learning cards. target_type=lexeme resolves initialized lexemes to entry IDs first; target_type=entry uses entry IDs directly. " +
        "remove keeps the knowledge entry and review logs. reset returns the card to New while keeping review logs. " +
        "Only perform the exact mutation explicitly confirmed by the user.",
      inputSchema: z.object({
        action: z.enum(["add", "remove", "reset"]),
        target_type: z.enum(["lexeme", "entry"]),
        ids: positiveIds,
      }),
      annotations: destructiveWrite,
    },
    async ({ action, target_type: targetType, ids }) =>
      handle(async () =>
        runMany(ids, async (id) => {
          const entryId = targetType === "lexeme" ? await api.entryIdForLexeme(id) : id;
          if (action === "add") return api.put(`/agent/learning/${entryId}`);
          if (action === "remove") return api.delete(`/agent/learning/${entryId}`);
          return api.post(`/agent/learning/${entryId}/reset`);
        }),
      ),
  );

  server.registerTool(
    "learning_correction",
    {
      title: "Correct today's learning feedback",
      description:
        "Targeted current-day corrections. reset_intraday keeps the first feedback and long-term FSRS state; " +
        "undo removes only the latest effective intraday feedback; correct_first replaces the first feedback from its stored snapshot. " +
        "correct_first requires rating=known|fuzzy|forgotten. Never retry a rejected correction blindly.",
      inputSchema: z.object({
        action: z.enum(["reset_intraday", "undo", "correct_first"]),
        entry_ids: positiveIds,
        rating: z.enum(["known", "fuzzy", "forgotten"]).optional(),
      }),
      annotations: destructiveWrite,
    },
    async ({ action, entry_ids: entryIds, rating }) =>
      handle(async () => {
        if (action === "correct_first" && !rating) {
          throw new Error("rating is required for correct_first");
        }
        return runMany(entryIds, (entryId) => {
          if (action === "reset_intraday") {
            return api.post(`/agent/learning/${entryId}/reset-intraday`);
          }
          if (action === "undo") {
            return api.post(`/agent/learning/${entryId}/undo`);
          }
          return api.post(`/agent/learning/${entryId}/correct-first`, { rating });
        });
      }),
  );

  server.registerTool(
    "article_query",
    {
      title: "Read saved articles",
      description:
        "Read-only access to the article-learning library. action=list returns folders and article summaries. action=get requires article_ids and returns full persisted articles with bookmark segments.",
      inputSchema: z.object({
        action: z.enum(["list", "get"]),
        article_ids: positiveIds.optional(),
      }),
      annotations: readOnly,
    },
    async ({ action, article_ids: articleIds }) =>
      handle(async () => {
        if (action === "list") return api.get("/reading");
        requireValues(articleIds, "article_ids");
        return runMany(articleIds, (articleId) => api.get(`/reading/articles/${articleId}`));
      }),
  );

  server.registerTool(
    "article_import",
    {
      title: "Import a reading article",
      description:
        "Persist one user-confirmed article. Inline bookmarks use [surface](headword). The backend rejects the whole import when a headword does not resolve to a complete entry. " +
        "Do not guess or silently rewrite failed bookmarks.",
      inputSchema: z.object({
        title: z.string().trim().min(1),
        body: z.string().trim().min(1),
        folder_id: z.number().int().positive().nullable().optional(),
      }),
      annotations: writesData,
    },
    async ({ title, body, folder_id: folderId }) =>
      handle(() => api.post("/reading/articles", { title, body, folder_id: folderId ?? null })),
  );

  return server;
}

function requireValues<T>(values: readonly T[] | undefined, name: string): asserts values is readonly T[] {
  if (!values?.length) throw new Error(`${name} is required for this action`);
}

async function handle(operation: () => Promise<unknown> | unknown) {
  try {
    const value = await operation();
    return {
      content: [{ type: "text" as const, text: JSON.stringify(value, null, 2) }],
    };
  } catch (error) {
    return {
      isError: true,
      content: [{ type: "text" as const, text: errorMessage(error) }],
    };
  }
}

function compactInitialization(value: unknown): unknown {
  if (!value || typeof value !== "object" || Array.isArray(value)) return value;
  const response = value as Record<string, unknown>;
  return {
    entry_id: response.entry_id,
    query_text: response.query_text,
    source: response.source,
    model: response.model,
    status: "initialized",
  };
}
