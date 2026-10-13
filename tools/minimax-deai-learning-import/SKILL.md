---
name: deai-learning-import
description: Resolve German surface forms from reading material, inspect De-AI-Hilfer vocabulary and learning state, and initialize or import user-confirmed words through the bundled remote CLI.
---

# De-AI-Hilfer learning import

This skill uses the bundled CLI at `<skill-root>/bin/deai`. Its remote API configuration is kept in `<skill-root>/deai.env` and must never be printed, embedded in prompts, or uploaded. Use `https://de.int.alan-studio.uk/api/v1` when the MiniMax command runtime can reach the private network; otherwise use the public fallback `https://de.alan-studio.uk/api/v1`. Browser reachability does not prove that the command runtime shares the same route.

For every command, run:

```bash
<skill-root>/bin/deai --env-file <skill-root>/deai.env ...
```

The CLI calls the remote De-AI-Hilfer API only. Never access a database directly.

## Article workflow

1. Extract useful surface forms and retain their local sentences in the task context only. The explicit article import below is the only exception.
2. Preserve occurrence boundaries. Identical spelling in multiple positions may represent different lexemes.
3. Resolve forms in one read-only batch:

```bash
printf '%s' '["ging","hätte"]' | <skill-root>/bin/deai --env-file <skill-root>/deai.env lexeme resolve --stdin
```

4. Use each sentence to judge candidates. Do not invent a candidate when the dictionary returns none.
5. Inspect selected `lexeme_id` values:

```bash
printf '%s' '[123,456]' | <skill-root>/bin/deai --env-file <skill-root>/deai.env lexeme inspect --stdin
```

6. Show a concise confirmation table with occurrence, proposed headword, part of speech, gloss, knowledge state, and learning state. Clearly separate ambiguous and missing items.
7. Do not write until the user explicitly confirms the selected headwords and actions in the current conversation.
8. Initialize only confirmed lexemes whose `knowledge.status` is `missing`:

```bash
<skill-root>/bin/deai --env-file <skill-root>/deai.env entry initialize 123 456 --async --idempotency-key confirmed-words-1 --yes
```

Poll the returned task with the same CLI/env-file prefix and `batch get TASK_ID`
until terminal; inspect each word result before continuing.

9. Add only successfully initialized or already-ready entries:

```bash
<skill-root>/bin/deai --env-file <skill-root>/deai.env learning add 123 --lexeme --yes
```

10. Re-inspect affected lexemes and report their actual final states.

## Article import

Only persist an article when the user explicitly asks. Bookmarks belong directly in the body as `[surface](headword)`. If a bookmark headword does not resolve to one complete entry, report the validation error and do not silently rewrite the article.

```bash
printf '%s' '{"title":"Reisen","body":"Ich plane meine [Reisen](Reise).","folder_id":null}' \
  | <skill-root>/bin/deai --env-file <skill-root>/deai.env article import --stdin --yes
```

## General asynchronous operation lists

Prefer `entry initialize ID... --async --idempotency-key KEY --yes` for
multiple confirmed words. It immediately returns a task ID; query it with
`batch get TASK_ID` every 3–5 seconds and inspect each result before reporting
completion or adding cards. Keep the existing skill-root/env-file prefix.

For a confirmed mixed list, `batch submit --stdin --yes` accepts:
`{idempotency_key,on_error?:"continue"|"stop",operations:[{id,depends_on?,operation}]}`.
Supported operation actions and parameters:

- `lexeme_initialize`: `lexeme_ids` array, optional `model_override:{provider_name,model_id}`.
- `learning_membership`: `mutation:add|remove|reset`, optional `target_type:entry|lexeme` (default entry), `ids` array.
- `folder_create`: `name`, optional `parent_id`.
- `article_import`: `title,body`, optional `folder_id`.

Every ID is a positive integer or `{step,path}` pointing to an earlier
result: folder `/folder_id`, initialization `/results/0/value/entry_id`.
Steps run in order; a multi-word initialization is bounded concurrent.
`depends_on` requires whole steps to succeed; an individual successful word
of a partially failed step can be referenced separately. `on_error` defaults
to continue; stop skips later steps after a failure. Invalid references skip
the dependent operation. The batch is not an all-or-nothing transaction.

Reuse a stable idempotency key for identical uncertain submissions. Use
`batch retry TASK_ID --yes` for unfinished work on the same approved list,
and `batch list` to recover task IDs. The list cannot change during retry;
new content needs a new approved list/key. Successful writes are preserved.
Never guess new words or silently rewrite failed article bookmarks.

## Safety

- Treat `lexeme_id` and `entry_id` as opaque, database-local identifiers. Resolve them again in later tasks.
- Never trigger hidden AI fallback when candidate resolution has no reliable result. Ask the user to handle that item manually in the web app.
- Do not initialize every extracted word automatically. The user decides what is worth learning.
- Do not remove or reset learning progress unless the user explicitly requests that exact action.
- `learning remove` removes the card from the plan but retains its knowledge entry and review logs.
- `learning reset` returns the card to New while retaining review logs.
- Batch commands continue after per-item failures. Inspect every result and surface failures.

## Read-only learning diagnostics

```bash
<skill-root>/bin/deai --env-file <skill-root>/deai.env learning explain
<skill-root>/bin/deai --env-file <skill-root>/deai.env learning logs --date 2026-09-17 --limit 100
<skill-root>/bin/deai --env-file <skill-root>/deai.env learning logs --entry-id 2591
<skill-root>/bin/deai --env-file <skill-root>/deai.env learning plan
<skill-root>/bin/deai --env-file <skill-root>/deai.env learning reset-intraday 789 --yes
<skill-root>/bin/deai --env-file <skill-root>/deai.env learning undo 789 --yes
<skill-root>/bin/deai --env-file <skill-root>/deai.env learning correct-first 789 --rating fuzzy --yes
```

`learning logs` returns the persisted rating timeline for the selected
business day. `learning plan` returns the reconstructible current-day plan,
including long-term due cards and unfinished intraday recoveries. Both commands
are read-only. The plan does not expose the exact in-memory ordering of an
already open browser session.

Use `learning explain` when the Agent needs the current V3 scheduler rules,
the first-feedback versus intraday boundary, or the current correction
operations.

Correction commands are targeted writes. `reset-intraday` preserves the first
feedback and long-term FSRS state, `undo` removes only the latest effective
intraday feedback, and `correct-first` requires a stored progress snapshot
before it recomputes long-term FSRS. Inspect each response and do not retry a
rejected correction blindly. Historical logs without progress snapshots are
rejected by `correct-first` rather than guessed.
