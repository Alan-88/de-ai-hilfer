---
name: deai-learning-import
description: Resolve German surface forms from articles, inspect De-AI-Hilfer vocabulary and learning state, and initialize or import user-confirmed words through the repository CLI. Use when Codex needs to process German reading material, check whether vocabulary already exists or is being learned, prepare a vocabulary import, or manage confirmed learning-plan entries without direct database access.
---

# De-AI-Hilfer learning import

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
./tools/deai entry initialize 123 --yes
```

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
