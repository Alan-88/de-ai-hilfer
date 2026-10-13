# MiniMax Code learning-import package

This package is self-contained except for Node.js 18 or later. It calls the public De-AI-Hilfer API and does not need the project repository, Docker, SSH, or database access.

## Contents

- `SKILL.md`: the learning-import workflow for MiniMax Code
- `bin/deai`: the dependency-free Node.js CLI
- `deai.env.example`: public API configuration template

## Setup

Place this folder where MiniMax Code loads local skills. Copy `deai.env.example` to `deai.env` beside `SKILL.md`, then fill in the Agent API token. Keep `deai.env` local and do not include it in prompts, chat exports, or source control; the repository package intentionally contains only `deai.env.example`.

The skill always calls:

```bash
<skill-root>/bin/deai --env-file <skill-root>/deai.env ...
```

`--url` and `--token` are available only for local diagnostics and override the file configuration. All mutation commands still require `--yes`.

Read-only GET requests use a 60-second timeout and retry transient network
failures twice. Write requests are not retried automatically, because a lost
connection can happen after the server has already accepted the write. If a
request still fails, the CLI reports the underlying DNS, TLS, socket, or
timeout cause. Use the internal URL when this command runtime shares the
private route; otherwise use the public URL. A browser reaching the site does
not prove that a background command has the same network namespace or proxy.

For long vocabulary generation, prefer `entry initialize ID... --async
--idempotency-key KEY --yes`, which returns a task ID immediately. `batch
submit --stdin --yes` submits a general ordered operation list including
initialization, learning membership, folder creation and article import.
Poll `batch get TASK_ID`; use `batch retry TASK_ID --yes` for unfinished work.
Keep the same key when an identical submission's response was lost.

`follow-up list/get` reads saved questions and answers. `follow-up create/edit`
accepts Markdown JSON through stdin; edits require previously read
expected_question/expected_answer. `follow-up delete ID --yes` permanently
removes one record. Internal `follow-up generate/retry` returns an async task;
poll `batch get` and preserve the idempotency key for uncertain resubmissions.
