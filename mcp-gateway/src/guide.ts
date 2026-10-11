export const LEARNING_GUIDE = String.raw`# De-AI-Hilfer learning V3

This is the current learning state machine. Read this before inspecting or changing learning data.

## Identity and day boundary

- A learning card is a knowledge entry identified by entry_id. lexeme_id identifies a dictionary lexeme and is not interchangeable with entry_id.
- The business day uses Asia/Shanghai (UTC+8) and rolls over at 04:00 local time.

## First feedback versus intraday feedback

- The first rating for a card in one business day is the long-term decision. It calls FSRS once, updates long-term progress and due_date, and is logged with long_term_committed=true.
- Later ratings for the same card on that business day are intraday-only. They do not call FSRS again and do not change the long-term due date.
- New cards use the UI labels known / fuzzy / forgotten. Review cards use known / fuzzy / forgotten, where forgotten is the review-card wording for not remembering.

## Intraday rules

- The scheduler is time-first. A card normally waits until earliest_review_at; due_after_seen only orders cards that are already available in the same window.
- Delays are forgotten=45 seconds, fuzzy=2 minutes, known=1 minute.
- If the first rating was forgotten and a later rating is known, an extra 5-minute recovery delay is applied. This is the intentional short-term stabilization rule; there is no separate long silent-period rule.
- The active intraday workset target is 10 cards. This is a soft intake threshold, not a daily new-card limit. If the workset is full and no card is due, the scheduler may replay the nearest card as a count-based fallback.
- A card appears at most 6 times in one business day.
- Completion rules: first known ends that card for today; first fuzzy needs one later known; first forgotten needs two later consecutive known ratings. A fuzzy or forgotten rating resets the current known streak.

## Refresh and diagnostics

- Review logs are persisted, so a new browser session can recover unfinished intraday cards for the current business day.
- Recovery reconstructs card state and timing, but not the exact in-memory cursor/order of an already open browser session.
- learning logs is read-only and returns the persisted rating timeline.
- learning plan is read-only and returns a reconstructible current-day plan. It does not start a session or mutate the database.
- The browser V3 session endpoints are for the web learning flow. There is currently no Agent command for inspecting the live in-memory queue.

## Safe operation boundaries

- learning add/remove/reset are existing plan operations. learning reset resets the long-term learning card to New while retaining historical review logs; it is not an intraday-only reset.
- learning reset-intraday voids all effective later ratings for the current business day while keeping the first rating and long-term FSRS state.
- learning undo voids only the latest effective later rating for the current business day.
- learning correct-first replaces the first rating and recomputes long-term FSRS from the stored pre-feedback snapshot. It also invalidates open browser sessions for that business day.
- All corrections are append-only audit records. Historical logs created before snapshot support cannot be corrected automatically; the API rejects them instead of guessing.
- Do not expose scheduler constants as user-tunable parameters until real review logs justify changing them.
`;

export const SERVER_INSTRUCTIONS = String.raw`De-AI-Hilfer is the owner's article-first German learning system. It combines dictionary-form resolution, generated knowledge entries, saved reading articles and FSRS-based vocabulary review.

Use this mental model: a surface occurrence in text resolves to a dictionary lexeme; an initialized lexeme has a knowledge entry; a knowledge entry may have a learning card. lexeme_id and entry_id are not interchangeable.

Read operations may run directly. Before initializing entries, importing articles, changing learning membership or correcting review history, inspect the current state and obtain explicit user confirmation. Never guess a lexeme when resolution is ambiguous or missing, and never bulk-add every unfamiliar word from an article.

Call deai_guide with the relevant topic before a multi-step workflow or whenever the data model, article bookmark rules, learning scheduler, diagnostics or correction semantics are unclear.`;

export const DEAI_GUIDE_TOPICS = [
  "overview",
  "article_learning",
  "vocabulary_workflow",
  "learning_v3",
  "learning_diagnostics",
  "learning_corrections",
] as const;

export type DeaiGuideTopic = (typeof DEAI_GUIDE_TOPICS)[number];

interface DeaiGuide {
  version: string;
  topic: DeaiGuideTopic;
  summary: string;
  concepts: readonly string[];
  workflow: readonly string[];
  safety_rules: readonly string[];
  related_tools: readonly string[];
  details?: string;
}

const GUIDE_VERSION = "2026-10-07";

export const DEAI_GUIDES: Readonly<Record<DeaiGuideTopic, DeaiGuide>> = {
  overview: {
    version: GUIDE_VERSION,
    topic: "overview",
    summary:
      "De-AI-Hilfer supports an article-first German workflow: resolve words in context, build reusable knowledge entries, save occurrence-specific article bookmarks and review selected vocabulary with FSRS.",
    concepts: [
      "A surface occurrence is one exact position in source text. Identical spelling elsewhere is not automatically the same occurrence or lexeme.",
      "lexeme_id identifies a dictionary lexeme. entry_id identifies an initialized knowledge entry. A learning card belongs to an entry.",
      "The knowledge library stores reusable word analysis; the article library stores source text and bookmarks; the learning plan stores selected review cards.",
      "The owner normally learns vocabulary through articles rather than importing every unfamiliar word from a list.",
    ],
    workflow: [
      "Keep source sentences in the conversation and use them to disambiguate dictionary candidates.",
      "Resolve surface forms, inspect selected lexemes, and present uncertain or missing cases instead of guessing.",
      "After explicit confirmation, initialize missing knowledge entries and add only selected entries to the learning plan.",
      "When requested, import the article with validated occurrence-specific bookmarks.",
      "Use learning plan, logs and corrections only when the user asks about review state or a mistaken rating.",
    ],
    safety_rules: [
      "Read operations may run without confirmation; every write must match an explicit user request.",
      "Do not initialize, import, add, remove, reset or correct items speculatively.",
      "Do not treat lexeme_id and entry_id as interchangeable or stable outside the returned data.",
    ],
    related_tools: [
      "lexeme_query",
      "lexeme_initialize",
      "article_query",
      "article_import",
      "learning_query",
      "learning_membership",
      "learning_correction",
    ],
  },
  article_learning: {
    version: GUIDE_VERSION,
    topic: "article_learning",
    summary:
      "Article learning preserves the original text and marks selected occurrences with bookmarks after their headwords have been confirmed and initialized.",
    concepts: [
      "A bookmark is occurrence-specific. Never annotate every identical string merely because one occurrence was resolved.",
      "Bookmark syntax is [surface](headword): surface is the original displayed text and headword is the confirmed dictionary form.",
      "Context helps the agent choose a candidate but is not persisted through lexeme commands. The explicit article body is the only persisted source context.",
      "Article saving is atomic: every bookmark headword must resolve to one complete knowledge entry or the backend rejects the whole import.",
    ],
    workflow: [
      "Identify potentially useful occurrences while preserving each position and its local sentence.",
      "Batch-resolve their surface forms with lexeme_query(action=resolve).",
      "Use the local sentence to select the intended form-of candidate and keep ambiguous items separate.",
      "Inspect selected lexeme IDs with lexeme_query(action=inspect).",
      "Show the user the proposed headword, gloss, knowledge status and learning status for confirmation.",
      "Initialize only confirmed missing entries, then add only confirmed entries to learning when requested.",
      "Insert bookmarks only at the confirmed occurrences and import the final article after explicit confirmation.",
    ],
    safety_rules: [
      "Do not silently rewrite a bookmark that fails validation.",
      "Do not guess separable verbs, capitalization or ambiguous forms when candidates do not support the choice.",
      "Do not persist source sentences through vocabulary operations or assume every marked word belongs in the learning plan.",
    ],
    related_tools: ["lexeme_query", "lexeme_initialize", "learning_membership", "article_query", "article_import"],
  },
  vocabulary_workflow: {
    version: GUIDE_VERSION,
    topic: "vocabulary_workflow",
    summary:
      "Vocabulary operations separate dictionary candidate resolution, knowledge-entry inspection, AI initialization and learning-plan membership.",
    concepts: [
      "resolve accepts surface forms and returns ranked dictionary lexeme candidates, including form-of relationships and short glosses.",
      "inspect accepts lexeme IDs and returns the corresponding knowledge-entry and learning state without generating missing content.",
      "A lexeme may exist in the dictionary while its knowledge entry is still missing. Initialization creates the complete reusable entry through the AI pipeline.",
      "Initialization may take several minutes and is separate from adding the resulting entry to the learning plan.",
    ],
    workflow: [
      "Resolve one or more surface forms in a read-only batch.",
      "Use conversation context to choose candidates and ask the user when the result is ambiguous.",
      "Inspect selected lexemes to distinguish ready entries, missing entries and existing learning cards.",
      "Obtain explicit confirmation for the proposed headwords and actions.",
      "Initialize confirmed missing entries, inspect the returned result, and add only confirmed ready entries to learning.",
      "Re-inspect affected lexemes before reporting final state.",
    ],
    safety_rules: [
      "No reliable candidate means stop and ask the user to handle that word manually in the web app.",
      "Never claim initialization or learning membership succeeded without checking the returned result.",
      "Batch results may be partially successful; report each failed item instead of treating the batch as complete.",
    ],
    related_tools: ["lexeme_query", "lexeme_initialize", "learning_membership"],
  },
  learning_v3: {
    version: GUIDE_VERSION,
    topic: "learning_v3",
    summary:
      "The V3 scheduler commits the first daily rating to long-term FSRS and uses later ratings only for persisted intraday reinforcement.",
    concepts: [
      "The business day uses Asia/Shanghai and rolls over at 04:00.",
      "The first rating updates long-term FSRS once; later ratings change only the current business day's intraday state.",
      "Scheduling is time-first with a 10-card soft active workset and a count-based fallback when waiting would stall the session.",
      "Review logs persist unfinished intraday state across refreshed browser sessions, but not an open session's exact in-memory cursor.",
    ],
    workflow: [
      "Use learning_query(action=plan) for the reconstructible current-day plan.",
      "Use learning_query(action=logs) to inspect the actual rating timeline before diagnosing unexpected scheduling.",
      "Read the detailed state-machine document below before changing or explaining review state.",
    ],
    safety_rules: [
      "Do not infer the exact next browser card from the reconstructed plan.",
      "Do not expose or modify scheduler constants merely because one session feels unusual.",
      "Use correction operations rather than deleting or rewriting raw review logs.",
    ],
    related_tools: ["learning_query", "learning_membership", "learning_correction"],
    details: LEARNING_GUIDE,
  },
  learning_diagnostics: {
    version: GUIDE_VERSION,
    topic: "learning_diagnostics",
    summary:
      "Learning diagnostics distinguish membership, per-word status, persisted review history and the reconstructible daily plan.",
    concepts: [
      "list returns entries currently present in the learning plan.",
      "status inspects selected lexemes and includes their knowledge and learning state.",
      "logs returns persisted ratings and correction effects, optionally filtered by business date, entry or session.",
      "plan reconstructs today's due, new and unfinished intraday cards from durable state; it does not expose an open browser session's live queue cursor.",
    ],
    workflow: [
      "Start with status for a specific word or list for overall learning membership.",
      "Use plan to answer what is reconstructibly due today and when waiting cards become available.",
      "Use logs for the affected entry and business day when a card appeared, disappeared or advanced unexpectedly.",
      "Compare first-feedback long_term_committed state with later intraday ratings before recommending a correction.",
    ],
    safety_rules: [
      "Diagnostics are read-only and should precede any correction.",
      "Do not present plan ordering as the exact order of a currently open browser session.",
      "Do not start or mutate a browser learning session merely to inspect state.",
    ],
    related_tools: ["learning_query"],
  },
  learning_corrections: {
    version: GUIDE_VERSION,
    topic: "learning_corrections",
    summary:
      "Correction operations repair a mistaken current-day rating while preserving append-only review history and the intended FSRS boundary.",
    concepts: [
      "undo voids only the latest effective later intraday rating; it cannot undo the first rating.",
      "reset_intraday keeps the first rating and long-term FSRS result but voids all effective later ratings for the current business day.",
      "correct_first replaces the first rating and recomputes long-term FSRS from its stored pre-feedback snapshot.",
      "learning membership reset is different: it returns the whole long-term card to New while retaining historical logs.",
    ],
    workflow: [
      "Inspect status, today's logs and the reconstructed plan for the affected entry.",
      "Explain the exact effect of the proposed operation and obtain explicit user confirmation.",
      "Use undo for one mistaken later click, reset_intraday to restart later repetitions, or correct_first for a mistaken first rating.",
      "Inspect the response and refreshed state; never retry a rejected correction blindly.",
    ],
    safety_rules: [
      "Corrections are targeted writes and must name the affected entry and intended replacement explicitly.",
      "Historical first ratings without a stored snapshot cannot be corrected automatically; report the rejection instead of guessing.",
      "A successful first-rating correction invalidates open browser sessions for that business day, so the user must reopen learning.",
    ],
    related_tools: ["learning_query", "learning_correction", "learning_membership"],
  },
};
