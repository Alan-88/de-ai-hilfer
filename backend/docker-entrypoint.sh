#!/bin/sh
set -eu

DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-de_ai_hilfer}"

if [ -n "${DB_USER:-}" ] && [ -n "${DB_HOST:-}" ]; then
  if [ -n "${DB_PASSWORD:-}" ]; then
    export DATABASE_URL="postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
  else
    export DATABASE_URL="postgres://${DB_USER}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
  fi
fi

export SERVER_HOST="${SERVER_HOST:-0.0.0.0}"
export SERVER_PORT="${SERVER_PORT:-8000}"
export PROMPT_CONFIG_PATH="${PROMPT_CONFIG_PATH:-/app/config/prompts/default.yaml}"

exec /app/server
