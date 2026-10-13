import * as z from "zod/v4";

const id = z.number().int().positive();
const referencedId = z.union([id, z.object({ step: z.string().min(1).max(64), path: z.string().startsWith("/").max(256) }).strict()]);
export const batchOperation = z.discriminatedUnion("action", [
  z.object({
    action: z.literal("lexeme_initialize"), lexeme_ids: z.array(id).min(1).max(100),
    model_override: z.object({ provider_name: z.string().trim().min(1), model_id: z.string().trim().min(1) }).strict().optional(),
  }).strict(),
  z.object({
    action: z.literal("learning_membership"), mutation: z.enum(["add", "remove", "reset"]),
    target_type: z.enum(["entry", "lexeme"]).optional(), ids: z.array(referencedId).min(1).max(100),
  }).strict(),
  z.object({ action: z.literal("folder_create"), name: z.string().trim().min(1).max(240), parent_id: referencedId.nullable().optional() }).strict(),
  z.object({ action: z.literal("article_import"), title: z.string().trim().min(1).max(240), body: z.string().min(1), folder_id: referencedId.nullable().optional() }).strict(),
]);

export const batchSubmission = z.object({
  idempotency_key: z.string().min(1).max(128),
  on_error: z.enum(["continue", "stop"]).optional(),
  operations: z.array(z.object({
    id: z.string().regex(/^[a-zA-Z0-9_-]{1,64}$/),
    depends_on: z.array(z.string().min(1).max(64)).optional(), operation: batchOperation,
  }).strict()).min(1).max(100),
}).strict();
