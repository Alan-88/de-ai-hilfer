# De-AI-Hilfer

A German-learning retrieval system built with Rust, SvelteKit, PostgreSQL, and `pgvector`.

The project focuses on dictionary-centered query resolution rather than free-form generation. It combines lexeme/surface-form modeling, vector retrieval, cached analysis, and AI-assisted explanation to support word lookup, phrase attachment, and a lightweight review workflow.

## Stack

| Layer | Choice |
| --- | --- |
| Frontend | SvelteKit + TypeScript |
| Backend | Rust + Axum |
| Database | PostgreSQL + `pgvector` |
| AI Gateway | OpenAI-compatible API |
| Analysis Models | Gemini primary, GLM fallback |
| Embedding Model | `glm-embedding-3` |
| Infra | Docker Compose + nginx |

## Architecture

```text
User Query
  -> SvelteKit UI
  -> Axum API
  -> Multi-stage resolution
       -> exact hit
       -> lexeme / surface-form lookup
       -> fuzzy matching
       -> embedding-assisted inference
       -> AI-backed analysis
  -> cached response or streamed generation
  -> knowledge library / learning flow
```

## Retrieval Pipeline

The runtime path is organized as a staged lexical retrieval flow:

1. `Exact hit`
   Reuse existing knowledge entries and stable dictionary matches first.
2. `Lexeme / surface-form resolution`
   Normalize inflected forms and map them onto analyzable lexeme records.
3. `Fuzzy matching`
   Tolerate case variance, umlaut variance, and light misspellings.
4. `Embedding-assisted inference`
   Use lexeme embeddings to improve headword inference when lexical signals are weak.
5. `AI-backed analysis`
   Generate explanations only after the target lexeme is resolved as reliably as possible.

## Data Model

The dictionary layer is rebuilt from upstream Kaikki/Wiktionary-derived data into a query-oriented structure:

- `dictionary_raw_entries`
  Raw imported records retained for reconstruction and verification.
- `dictionary_lexemes`
  Canonical analyzable lexeme records.
- `dictionary_surface_forms`
  Surface forms, aliases, and form-to-lexeme mappings used for retrieval.
- `dictionary_lexeme_embeddings`
  Embeddings attached to lexemes instead of flattened headword rows.

The current local dataset includes roughly `93k+` lexeme embeddings.

## Key Engineering Decisions

### Lexeme-first retrieval instead of chunking-style RAG

Dictionary data has strong lexical structure: lemma, part of speech, inflection, alias, and form-of relations. A plain chunking pipeline loses those relations quickly. This project resolves the lexical target first, then lets the model analyze against dictionary facts.

### Cache-first runtime behavior

Known entries are returned directly. New or refreshed entries can use streamed analysis. This keeps repeated lookups fast while preserving an incremental generation path for misses.

### Separate runtime fallback from prewarm consistency

Runtime requests may fall back from Gemini to GLM to keep the product responsive. Batch prewarm jobs are stricter and avoid mixing outputs from different models into the persistent knowledge base.

### Dictionary-grounded generation

The model is used as an analyzer, not as the primary source of truth. Dictionary facts constrain the target and reduce drift in explanations, examples, and phrase usage notes.

## Features

- Search suggestions with exact, fuzzy, and lexeme-aware lookup
- Dictionary-backed word analysis and cached reuse
- Phrase attachment workflow for adding usage modules onto host entries
- Knowledge library with structured detail views
- Follow-up and review-oriented learning flow
- Docker-based local deployment

## Project Layout

- `frontend-web/`
  SvelteKit UI and client-side analysis rendering
- `backend/`
  Axum API, retrieval logic, AI integration, data import, and prewarm tooling
- `compose.yaml`
  Container entrypoint for frontend, backend, and database wiring
- `assets/dictionary/`
  Notes for dictionary source handling

## Run

```bash
docker compose up -d
```

The repository keeps the application source, container setup, migrations, and prompt/runtime logic. Large raw dictionary dumps, local logs, private environment files, and internal working notes are intentionally excluded.

## Notes

- The project depends on external dictionary source material and local preprocessing steps that are not bundled in the repository.
- Some AI gateway settings are environment-specific and are expected to be provided locally.
- The repository is maintained as a technical showcase and working codebase rather than a turnkey public product.
