---
name: deai-learning-import
description: Resolve German surface forms, inspect De-AI-Hilfer vocabulary and learning state, import confirmed words, and manage saved follow-up answers through the repository CLI. Use for German reading imports, learning-plan operations, or reading, editing, saving and retrying vocabulary follow-ups without direct database access.
---

# De-AI-Hilfer learning import

## Follow-up records

For user-requested follow-up management, read `docs/api/follow-ups.md` and use
`follow-up list ENTRY_ID` / `get FOLLOW_UP_ID` before writes. External answers
can be saved with `follow-up create --stdin --idempotency-key KEY --yes` or
edited with `follow-up edit ID --stdin --yes`; edits include the previously
read expected_question/expected_answer. `delete ID --yes` deletes that record
permanently. `generate --stdin --idempotency-key KEY --yes` and
`retry ID --idempotency-key KEY --yes` return async tasks; poll `batch get`.
Do not treat a failed generation as a valid answer or silently overwrite an
existing answer. IDs identify records under knowledge entry_id, not lexemes.

Run the repository CLI from the repository root:

```bash
./tools/deai ...
```

Let the CLI read `DEAI_API_URL` and `DEAI_API_TOKEN` from the process environment or the repository-local `backend/.env`. Never access the database directly.

## Article workflow

1. Extract potentially useful surface forms with their local sentences. Keep sentences in the task context only; never send or persist them through vocabulary commands. The explicit reading-article import below is the only exception.
2. Preserve occurrence boundaries. Identical spelling in multiple positions may represent different lexemes.
3. Resolve forms in one read-only batch:

```bash
printf '%s' '["ging","hätte"]' | ./tools/deai lexeme resolve --stdin
```

4. Use each sentence to judge the candidates. Do not invent a candidate when the dictionary returns none.
5. Inspect selected `lexeme_id` values:

```bash
printf '%s' '[123,456]' | ./tools/deai lexeme inspect --stdin
```

6. Show the user a concise confirmation table containing occurrence, proposed headword, part of speech, short gloss, knowledge status, and learning status. Clearly separate ambiguous and missing items.
7. Perform no write until the user explicitly confirms the proposed headwords and actions in the current conversation.
8. Initialize only confirmed `knowledge.status == missing` lexemes:

```bash
./tools/deai entry initialize 123 456 --async --idempotency-key confirmed-words-1 --yes
```

The response is a queued task, not completed entries. Retain its `id`, poll
`./tools/deai batch get TASK_ID` every 3–5 seconds until terminal and inspect
every per-word result. Do not split long generation into repeated single-word
writes. Reuse the same key for uncertain resubmissions.

9. Add only successfully initialized or already-ready entries:

```bash
./tools/deai learning add 123 --lexeme --yes
```

10. Re-inspect the affected lexemes and report actual final states. Do not claim success from an earlier command alone.

## Reading article import

Only persist an article when the user explicitly requests it. Bookmarks belong directly in the body as `[surface](headword)`, and the backend rejects the whole import if any headword does not resolve to one complete entry.

```bash
./tools/deai article list
./tools/deai article get 123
printf '%s' '{"title":"Reisen","body":"Ich plane meine [Reisen](Reise).","folder_id":null}' \
  | ./tools/deai article import --stdin --yes
```

Do not guess or silently rewrite a failing bookmark. Report the backend validation error and leave the article unpersisted.

## Safety

- Treat `lexeme_id` and `entry_id` as opaque database-local identifiers. Resolve them again in later tasks.
- Never trigger hidden AI fallback when resolution has no reliable candidate. Ask the user to handle that item manually in the web app.
- Do not initialize every extracted word automatically. The user decides what is worth learning.
- Do not remove or reset learning progress unless the user explicitly requests that exact action.
- `learning remove` removes the card from the plan but keeps its knowledge entry and review logs.
- `learning reset` returns the card to New state and keeps historical review logs.
- Batch commands continue after per-item failures. Inspect every returned item and surface failures; do not treat a partially successful batch as complete.
- When candidate matching is unexpectedly noisy, misses an obvious target, or ranks the contextually correct target poorly, append a compact case to `docs/iterations/agent-learning-import-anomalies.md`. Record only the surface form, candidate behavior, and chosen headword; never persist the source sentence. Do not add matching heuristics from a single case.

## General asynchronous operation lists

For an approved list of combined operations, use `./tools/deai batch submit
--stdin --yes` with one JSON object containing `idempotency_key`, optional
`on_error: continue|stop` and `operations: [{id,depends_on?,operation}]`.
Operations run in order and are not restricted to articles. Supported actions
are `lexeme_initialize`, `learning_membership` (add/remove/reset),
`folder_create` and `article_import`. Follow `docs/api/agent-batches.md` for the
exact schema. IDs can reference earlier results via `{step,path}` JSON pointers.

Read progress with `batch get TASK_ID` or `batch list`. `batch retry TASK_ID
--yes` retries only unfinished work on the original list; successful writes
have transactional receipts and must not be repeated in a new task. A changed
list requires a new key and must remain within the user's confirmed actions.
`queued/running` are not completion; `completed_with_errors` requires inspecting
failed/skipped steps. Do not invent candidates or rewrite failed bookmarks.

## Other commands

```bash
./tools/deai learning explain
./tools/deai learning list
./tools/deai learning status 123 456
./tools/deai learning logs --date 2026-09-17 --limit 100
./tools/deai learning logs --entry-id 2591
./tools/deai learning plan
./tools/deai learning reset-intraday 789 --yes
./tools/deai learning undo 789 --yes
./tools/deai learning correct-first 789 --rating fuzzy --yes
./tools/deai learning remove 789 --yes
./tools/deai learning reset 789 --yes
```

`learning logs` is read-only and returns the persisted rating timeline for the
current business day by default. Use `--date`, `--entry-id`, or `--session-id`
to narrow it. `learning plan` is also read-only: it returns the reconstructible
current-day plan, including long-term due cards and unfinished intraday
recoveries. Do not start a web learning session only to inspect the plan.

Run `learning explain` when the Agent needs the current V3 scheduler rules,
the distinction between first and intraday feedback, or the boundary between
existing plan reset and current-day correction operations.

Correction commands are targeted write operations. `reset-intraday` keeps the
first feedback and long-term FSRS state but removes effective later feedback;
`undo` removes only the latest effective intraday feedback; `correct-first`
changes the first feedback and recomputes long-term FSRS only when a stored
progress snapshot makes that correction safe. Always inspect the response and
do not retry a rejected correction blindly. Logs created before snapshot
support are intentionally rejected by `correct-first`.

The plan is not a snapshot of the in-memory order of an already open browser
session. It is the backend's safe reconstruction from learning progress and
review logs.
