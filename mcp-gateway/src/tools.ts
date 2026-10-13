import { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";
import { randomUUID } from "node:crypto";
import { batchSubmission } from "./batch.js";

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
    { name: "de-ai-hilfer", version: "0.4.0" },
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
        "Submit all user-confirmed missing lexemes as one asynchronous initialization job. Returns a task ID immediately, not finished entries. " +
        "Poll batch_query(action=get,batch_id=returned id) for per-word results; do not split into one call per word or resubmit while it is running. " +
        "Supply the same idempotency_key when repeating an uncertain submission. Do not call until the user has confirmed the selected headwords. " +
        "provider_name and model_id must be supplied together or both omitted.",
      inputSchema: z.object({
        lexeme_ids: positiveIds,
        provider_name: z.string().trim().min(1).optional(),
        model_id: z.string().trim().min(1).optional(),
        idempotency_key: z.string().min(1).max(128).optional(),
      }),
      annotations: writesData,
    },
    async ({ lexeme_ids: lexemeIds, provider_name: providerName, model_id: modelId, idempotency_key: key }) =>
      handle(async () => {
        if (Boolean(providerName) !== Boolean(modelId)) {
          throw new Error("provider_name and model_id must be supplied together");
        }
        return api.post("/agent/batches", {
          idempotency_key: key ?? randomUUID(),
          operations: [{ id: "initialize", operation: {
            action: "lexeme_initialize", lexeme_ids: lexemeIds,
            ...(providerName ? { model_override: { provider_name: providerName, model_id: modelId } } : {}),
          } }],
        });
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

  server.registerTool("follow_up_query", {
    title:"Read saved follow-ups",
    description:"Read saved question/answer records. action=list requires entry_id; action=get requires follow_up_id. IDs refer to saved records, not lexemes. Read the current record before editing or retrying.",
    inputSchema:z.object({action:z.enum(["list","get"]),entry_id:z.number().int().positive().optional(),follow_up_id:z.number().int().positive().optional()}),
    annotations:readOnly,
  }, async ({action,entry_id:entryId,follow_up_id:id}) => handle(() => {
    if (action === "list") {if (!entryId) throw new Error("entry_id is required"); return api.get(`/agent/follow-ups?entry_id=${entryId}`);}
    if (!id) throw new Error("follow_up_id is required");
    return api.get(`/agent/follow-ups/${id}`);
  }));

  server.registerTool("follow_up_write", {
    title:"Save, edit or delete a follow-up",
    description:"Persist an externally generated Markdown answer without calling the internal model. action=create requires entry_id,question,answer,idempotency_key; reuse the key for identical uncertain submissions. action=edit requires follow_up_id,question,answer,expected_question,expected_answer from a prior read; stale edits are rejected. action=delete requires follow_up_id and permanently deletes that one record. Only perform user-authorized writes; this does not initialize entries or change learning progress.",
    inputSchema:z.discriminatedUnion("action",[
      z.object({action:z.literal("create"),entry_id:z.number().int().positive(),question:z.string().trim().min(1).max(4000),answer:z.string().trim().min(1).max(100000),idempotency_key:z.string().min(1).max(128)}).strict(),
      z.object({action:z.literal("edit"),follow_up_id:z.number().int().positive(),question:z.string().trim().min(1).max(4000),answer:z.string().trim().min(1).max(100000),expected_question:z.string(),expected_answer:z.string()}).strict(),
      z.object({action:z.literal("delete"),follow_up_id:z.number().int().positive()}).strict(),
    ]),annotations:destructiveWrite,
  }, async (request) => handle(() => {
    if (request.action === "create") return api.post("/agent/follow-ups",{entry_id:request.entry_id,question:request.question,answer:request.answer,idempotency_key:request.idempotency_key});
    if (request.action === "delete") return api.delete(`/agent/follow-ups/${request.follow_up_id}`);
    return api.put(`/agent/follow-ups/${request.follow_up_id}`,{question:request.question,answer:request.answer,expected_question:request.expected_question,expected_answer:request.expected_answer});
  }));

  server.registerTool("follow_up_generate", {
    title:"Generate or retry a follow-up asynchronously",
    description:"Run the internal read-only vocabulary agent. action=create requires entry_id and question. action=retry requires follow_up_id; the saved question is reused and the existing answer is replaced only after successful generation. Returns a queued batch ID immediately; poll batch_query. Failure keeps the existing answer. Reuse idempotency_key for identical uncertain submissions. provider_name and model_id must be provided together.",
    inputSchema:z.object({action:z.enum(["create","retry"]),entry_id:z.number().int().positive().optional(),question:z.string().trim().min(1).max(4000).optional(),follow_up_id:z.number().int().positive().optional(),idempotency_key:z.string().min(1).max(128),provider_name:z.string().trim().min(1).optional(),model_id:z.string().trim().min(1).optional()}),
    annotations:destructiveWrite,
  }, async ({action,entry_id:entryId,question,follow_up_id:id,idempotency_key:key,provider_name:provider,model_id:model}) => handle(async () => {
    if (Boolean(provider) !== Boolean(model)) throw new Error("provider_name and model_id must be supplied together");
    if (action === "retry") {
      if (!id) throw new Error("follow_up_id is required");
      const saved = await api.get(`/agent/follow-ups/${id}`) as {entry_id:number;question:string};
      entryId = saved.entry_id; question = saved.question;
    }
    if (!entryId || !question) throw new Error("entry_id and question are required");
    return api.post("/agent/batches",{idempotency_key:key,operations:[{id:"followup",operation:{action:"follow_up_generate",entry_id:entryId,question,...(action === "retry" ? {retry_id:id} : {}),...(provider ? {model_override:{provider_name:provider,model_id:model}} : {})}}]});
  }));

  server.registerTool("batch_submit", {
    title: "Submit an asynchronous batch of operations",
    description: "Submit one user-approved list of business operations, executed in order in the background. Returns a task ID immediately. " +
      "Supports vocabulary initialization, learning add/remove/reset, folder creation, article import and follow_up_generate in any combination. " +
      "Use numeric IDs or {step,path} JSON-pointer references to earlier results, e.g. {step:'folder',path:'/folder_id'} or {step:'words',path:'/results/0/value/entry_id'}. " +
      "depends_on requires earlier steps to succeed; successful items in a partially failed step can still be referenced individually. " +
      "on_error defaults to continue; stop skips later steps after a failure. Reuse idempotency_key for identical resubmissions. " +
      "Every mutation must match the approved list; do not choose new candidates or rewrite failed article bookmarks during execution.",
    inputSchema: batchSubmission,
    annotations: destructiveWrite,
  }, async (request) => handle(() => api.post("/agent/batches", request)));

  server.registerTool("batch_query", {
    title: "Read background batch progress",
    description: "Read-only task progress. action=get requires batch_id; action=list returns recent tasks. " +
      "queued/running are unfinished; succeeded/completed_with_errors are terminal. Inspect every step and item result. Poll every 3–5 seconds, not in a tight loop. " +
      "Use the returned task ID instead of submitting the same work again.",
    inputSchema: z.object({action:z.enum(["get","list"]),batch_id:z.string().min(1).optional(),limit:z.number().int().min(1).max(100).optional()}),
    annotations: readOnly,
  }, async ({action,batch_id: id,limit}) => handle(() => {
    if (action === "list") return api.get(`/agent/batches?limit=${limit ?? 20}`);
    if (!id) throw new Error("batch_id is required for get");
    return api.get(`/agent/batches/${encodeURIComponent(id)}`);
  }));

  server.registerTool("batch_retry", {
    title: "Retry unfinished batch operations",
    description: "Retry failed/skipped operations of the same approved task; successful operations and committed mutations are preserved. " +
      "The operation list cannot be changed. Active jobs cannot be retried. Fix permanent validation errors in a separately approved new task instead.",
    inputSchema: z.object({batch_id:z.string().min(1)}),
    annotations: destructiveWrite,
  }, async ({batch_id:id}) => handle(() => api.post(`/agent/batches/${encodeURIComponent(id)}/retry`)));

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
