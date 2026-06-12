#!/usr/bin/env bash

# Long-running staged A/B/C backfill worker.
#
# Example:
#   tmux new-session -d -s de_backfill_worker \
#     "cd /path/to/backend && BACKFILL_TARGET_NEW=200 BACKFILL_MAX_SECONDS=36000 bash scripts/staged_backfill_worker.sh 2>&1 | tee -a tmp/staged_structured_backfill/worker.log"
#
# Safety model:
# - Only writes entries that reached ready/ after A/B/C and quality gates passed.
# - Reuses A/B cache for run_c retries; does not rerun A/B after C-only failure.
# - Defers transient C failures (timeout/rate limit/upstream) and lets the runner retry after cooldown.
# - Leaves only non-transient failures in tmp/staged_structured_backfill/failed for review.

set -u

QUEUE_DIR="${STAGED_BACKFILL_QUEUE_DIR:-tmp/staged_structured_backfill}"
STATE_DIR="${BACKFILL_WORKER_STATE_DIR:-$QUEUE_DIR/worker_state}"
TARGET_NEW="${BACKFILL_TARGET_NEW:-100}"
MAX_SECONDS="${BACKFILL_MAX_SECONDS:-0}"
BATCH_SIZE="${BACKFILL_BATCH_SIZE:-5}"
PREPARE_SPACING_SECS="${BACKFILL_PREPARE_SPACING_SECS:-45}"
PREPARE_CONCURRENCY="${BACKFILL_PREPARE_CONCURRENCY:-4}"
C_COOLDOWN_SECS="${BACKFILL_C_COOLDOWN_SECS:-45}"
BATCH_COOLDOWN_SECS="${BACKFILL_BATCH_COOLDOWN_SECS:-120}"
IDLE_SLEEP_SECS="${BACKFILL_IDLE_SLEEP_SECS:-300}"
DEFERRED_COOLDOWN_SECS="${BACKFILL_DEFERRED_COOLDOWN_SECS:-1800}"
STRUCTURE_MODEL="${BACKFILL_STRUCTURE_MODEL:-gpt-4o-mini}"
C_CONCURRENCY="${BACKFILL_C_CONCURRENCY:-4}"
C_RETRIES="${BACKFILL_C_RETRIES:-4}"
C_RETRY_INITIAL_SECS="${BACKFILL_C_RETRY_INITIAL_SECS:-120}"
C_RETRY_MAX_SECS="${BACKFILL_C_RETRY_MAX_SECS:-600}"

timestamp() {
  date '+%Y-%m-%dT%H:%M:%S%z'
}

log() {
  echo "[$(timestamp)] $*"
}

ensure_dirs() {
  mkdir -p "$QUEUE_DIR/ab" "$QUEUE_DIR/ready" "$QUEUE_DIR/applied" "$QUEUE_DIR/failed" "$QUEUE_DIR/deferred" \
    "$QUEUE_DIR/reports" "$STATE_DIR/attempts"
}

coverage_count() {
  docker exec PostgreSQL psql -U server -d de_ai_hilfer -At -c "
    select count(*) filter (
      where jsonb_typeof(analysis->'structured'->'grammar_branches') = 'array'
        and jsonb_array_length(analysis->'structured'->'grammar_branches') > 0
    )
    from knowledge_entries;
  " | tr -d '[:space:]'
}

coverage_total() {
  docker exec PostgreSQL psql -U server -d de_ai_hilfer -At -c \
    "select count(*) from knowledge_entries;" | tr -d '[:space:]'
}

report_field() {
  local report="$1"
  local field="$2"
  if [ -n "$report" ] && [ -f "$report" ]; then
    jq -r "$field" "$report" 2>/dev/null || echo "0"
  else
    echo "0"
  fi
}

run_stage() {
  local output status report
  output="$("$@" 2>&1)"
  status=$?
  printf '%s\n' "$output"
  report="$(printf '%s\n' "$output" | awk '/tmp\/staged_structured_backfill\/reports\/.*\.json$/ { path=$0 } END { print path }')"
  LAST_REPORT="$report"
  return "$status"
}

apply_ready() {
  run_stage env \
    STAGED_BACKFILL_COMMAND=apply_ready \
    STAGED_BACKFILL_LIMIT=25 \
    cargo run --bin staged_structured_backfill
}

select_new_words() {
  local existing="$STATE_DIR/existing_words.txt"
  local candidates="$STATE_DIR/candidate_words.txt"
  : > "$existing"

  find "$QUEUE_DIR/ab" "$QUEUE_DIR/ready" "$QUEUE_DIR/applied" "$QUEUE_DIR/failed" "$QUEUE_DIR/deferred" \
    -maxdepth 1 -type f -name "*.json" -print0 2>/dev/null \
    | xargs -0 jq -r ".query_text? // empty" 2>/dev/null \
    | awk "NF" \
    | sort -u > "$existing" || true

  docker exec PostgreSQL psql -U server -d de_ai_hilfer -At -c "
    select query_text
    from knowledge_entries
    where entry_type <> 'PHRASE'
      and analysis ? 'markdown'
      and length(coalesce(analysis->>'markdown','')) > 0
      and not coalesce(
        jsonb_typeof(analysis->'structured'->'grammar_branches') = 'array'
        and jsonb_array_length(analysis->'structured'->'grammar_branches') > 0,
        false
      )
    order by id desc
    limit 1000;
  " > "$candidates"

  grep -Fvx -f "$existing" "$candidates" | head -n "$BATCH_SIZE" | paste -sd, -
}

prepare_words() {
  local words="$1"
  if [ -z "$words" ]; then
    return 0
  fi
  log "prepare_ab words=$words"
  run_stage env \
    STAGED_BACKFILL_COMMAND=prepare_ab \
    STAGED_BACKFILL_WORDS="$words" \
    STAGED_BACKFILL_LIMIT="$BATCH_SIZE" \
    STAGED_BACKFILL_REQUEST_SPACING_SECS="$PREPARE_SPACING_SECS" \
    STAGED_BACKFILL_PREPARE_CONCURRENCY="$PREPARE_CONCURRENCY" \
    cargo run --bin staged_structured_backfill
}

run_one_c() {
  run_stage env \
    STAGED_BACKFILL_COMMAND=run_c \
    STAGED_BACKFILL_LIMIT="$BATCH_SIZE" \
    STAGED_BACKFILL_REQUEST_SPACING_SECS=0 \
    STAGED_BACKFILL_C_CONCURRENCY="$C_CONCURRENCY" \
    STAGED_BACKFILL_C_RETRIES="$C_RETRIES" \
    STAGED_BACKFILL_C_RETRY_INITIAL_SECS="$C_RETRY_INITIAL_SECS" \
    STAGED_BACKFILL_C_RETRY_MAX_SECS="$C_RETRY_MAX_SECS" \
    STAGED_BACKFILL_DEFERRED_COOLDOWN_SECS="$DEFERRED_COOLDOWN_SECS" \
    STAGED_BACKFILL_STRUCTURE_MODEL="$STRUCTURE_MODEL" \
    cargo run --bin staged_structured_backfill
}

run_c_cycle() {
  local cases successes failures
  log "run_c batch_size=$BATCH_SIZE concurrency=$C_CONCURRENCY model=$STRUCTURE_MODEL"
  run_one_c || true
  cases="$(report_field "$LAST_REPORT" '.cases | length')"
  successes="$(report_field "$LAST_REPORT" '.success_count // 0')"
  failures="$(report_field "$LAST_REPORT" '.failed_count // 0')"
  log "run_c report=$LAST_REPORT cases=$cases success=$successes failed=$failures"
  apply_ready || true
  if [ "$cases" = "0" ]; then
    log "run_c found no pending A/B artifacts"
    return 0
  fi
  sleep "$C_COOLDOWN_SECS"
}

file_age_secs() {
  local file="$1"
  local now mtime
  now="$(date +%s)"
  if stat -f %m "$file" >/dev/null 2>&1; then
    mtime="$(stat -f %m "$file")"
  else
    mtime="$(stat -c %Y "$file")"
  fi
  echo $((now - mtime))
}

release_cooled_deferred() {
  local released deferred_file age
  released=0
  for deferred_file in "$QUEUE_DIR"/deferred/*.json; do
    [ -e "$deferred_file" ] || break
    age="$(file_age_secs "$deferred_file")"
    if [ "$age" -ge "$DEFERRED_COOLDOWN_SECS" ]; then
      rm -f "$deferred_file"
      released=$((released + 1))
    fi
  done
  log "deferred release cycle released=$released"
}

print_queue_snapshot() {
  log "queue snapshot"
  find "$QUEUE_DIR/ready" "$QUEUE_DIR/deferred" "$QUEUE_DIR/failed" -maxdepth 1 -type f -print | sort || true
}

main() {
  local start_time start_count total current_count added words elapsed
  ensure_dirs
  start_time="$(date +%s)"
  start_count="$(coverage_count)"
  total="$(coverage_total)"
  log "worker start coverage=$start_count/$total target_new=$TARGET_NEW max_seconds=$MAX_SECONDS model=$STRUCTURE_MODEL"

  while true; do
    current_count="$(coverage_count)"
    added=$((current_count - start_count))
    elapsed=$(($(date +%s) - start_time))
    log "cycle coverage=$current_count/$total added=$added elapsed=${elapsed}s"

    if [ "$TARGET_NEW" -gt 0 ] && [ "$added" -ge "$TARGET_NEW" ]; then
      log "target reached"
      break
    fi
    if [ "$MAX_SECONDS" -gt 0 ] && [ "$elapsed" -ge "$MAX_SECONDS" ]; then
      log "max seconds reached"
      break
    fi

    apply_ready || true
    release_cooled_deferred
    run_c_cycle

    words="$(select_new_words)"
    if [ -n "$words" ]; then
      prepare_words "$words" || true
      run_c_cycle
    else
      log "no new eligible words; sleeping ${IDLE_SLEEP_SECS}s"
      sleep "$IDLE_SLEEP_SECS"
    fi

    print_queue_snapshot
    log "batch cooldown ${BATCH_COOLDOWN_SECS}s"
    sleep "$BATCH_COOLDOWN_SECS"
  done

  apply_ready || true
  current_count="$(coverage_count)"
  log "worker finished coverage=$current_count/$total added=$((current_count - start_count))"
  print_queue_snapshot
}

main "$@"
