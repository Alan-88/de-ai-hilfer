#!/usr/bin/env bash

# Normalize existing structured grammar branches without rerunning Model A/B/C.
#
# Default mode is dry-run:
#   bash scripts/normalize_transitive_government.sh
#
# Apply mode:
#   APPLY=true bash scripts/normalize_transitive_government.sh
#
# Rule:
# - verb branch
# - grammar.transitivity is transitive or both
# - grammar.governs_cases is an empty array
# => grammar.governs_cases = ["accusative"]

set -euo pipefail

POSTGRES_CONTAINER="${POSTGRES_CONTAINER:-PostgreSQL}"
DB_USER="${DB_USER:-server}"
DB_NAME="${DB_NAME:-de_ai_hilfer}"
APPLY="${APPLY:-false}"

psql_exec() {
  docker exec "$POSTGRES_CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" "$@"
}

read -r -d '' CANDIDATE_WHERE <<'SQL' || true
jsonb_typeof(analysis->'structured'->'grammar_branches') = 'array'
and exists (
  select 1
  from jsonb_array_elements(analysis->'structured'->'grammar_branches') branch
  where branch->>'pos' = 'verb'
    and branch->'grammar'->>'transitivity' in ('transitive', 'both')
    and jsonb_typeof(branch->'grammar'->'governs_cases') = 'array'
    and jsonb_array_length(branch->'grammar'->'governs_cases') = 0
)
SQL

echo "normalize_transitive_government: apply=$APPLY"
echo

echo "Candidate rows:"
psql_exec -c "
  select count(*) as rows
  from knowledge_entries
  where $CANDIDATE_WHERE;
"

echo "Candidate branches:"
psql_exec -c "
  with branches as (
    select
      query_text,
      branch->>'selector' as selector,
      branch->'grammar'->>'transitivity' as transitivity,
      branch->'grammar'->'governs_cases' as governs_cases,
      branch->'meanings'->0->>'zh' as zh
    from knowledge_entries
    cross join lateral jsonb_array_elements(analysis->'structured'->'grammar_branches') branch
    where jsonb_typeof(analysis->'structured'->'grammar_branches') = 'array'
  )
  select query_text, selector, transitivity, governs_cases, zh
  from branches
  where transitivity in ('transitive', 'both')
    and jsonb_typeof(governs_cases) = 'array'
    and jsonb_array_length(governs_cases) = 0
  order by query_text, selector
  limit 50;
"

if [ "$APPLY" != "true" ]; then
  echo
  echo "Dry-run only. Re-run with APPLY=true to update matching structured branches."
  exit 0
fi

echo
echo "Applying normalization..."
psql_exec -v ON_ERROR_STOP=1 -c "
  with rewritten as (
    select
      id,
      jsonb_agg(
        case
          when branch->>'pos' = 'verb'
            and branch->'grammar'->>'transitivity' in ('transitive', 'both')
            and jsonb_typeof(branch->'grammar'->'governs_cases') = 'array'
            and jsonb_array_length(branch->'grammar'->'governs_cases') = 0
          then jsonb_set(branch, '{grammar,governs_cases}', '[\"accusative\"]'::jsonb, true)
          else branch
        end
        order by ordinality
      ) as grammar_branches
    from knowledge_entries
    cross join lateral jsonb_array_elements(analysis->'structured'->'grammar_branches')
      with ordinality as branches(branch, ordinality)
    where $CANDIDATE_WHERE
    group by id
  )
  update knowledge_entries entry
  set
    analysis = jsonb_set(entry.analysis, '{structured,grammar_branches}', rewritten.grammar_branches, true),
    updated_at = now()
  from rewritten
  where entry.id = rewritten.id;
"

echo
echo "Remaining candidate rows:"
psql_exec -c "
  select count(*) as rows
  from knowledge_entries
  where $CANDIDATE_WHERE;
"
