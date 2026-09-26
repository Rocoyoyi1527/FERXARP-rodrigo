#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
umask 077

# This project and its volumes exist only for this invocation.
export COMPOSE_PROJECT_NAME="ferxarp_ci_${GITHUB_RUN_ID:-$$}"
export POSTGRES_DB="${POSTGRES_DB:-ferxarp_ci}"
export POSTGRES_USER="${POSTGRES_USER:-ferxarp}"
export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-ci-only-password}"
export JWT_SECRET="${JWT_SECRET:-ci-only-jwt-secret-for-ephemeral-tests}"
export FERXARP_ADMIN_EMAIL="${FERXARP_ADMIN_EMAIL:-admin-ci@example.invalid}"
export FERXARP_ADMIN_PASSWORD="${FERXARP_ADMIN_PASSWORD:-ci-only-admin-password}"
export BACKEND_ENV_FILE="$(mktemp /tmp/ferxarp-ci-backend-env.XXXXXX)"
unset GROQ_API_KEY
printf 'JWT_SECRET=%s\n' "$JWT_SECRET" > "$BACKEND_ENV_FILE"

cleanup() {
  local result=$?
  trap - EXIT
  if (( result != 0 )); then
    mkdir -p ci-artifacts
    docker compose logs --no-color --tail 100 postgres chromadb backend frontend > ci-artifacts/deploy.log 2>&1 || true
  fi
  if ! docker compose down -v --remove-orphans; then
    result=1
  fi
  rm -f "$BACKEND_ENV_FILE"
  exit "$result"
}
trap cleanup EXIT

docker compose config --quiet
docker compose build
docker compose up -d --wait --wait-timeout 180 postgres chromadb
docker compose run --rm --no-deps backend /app/migrate
docker compose run --rm --no-deps -e FERXARP_ADMIN_EMAIL -e FERXARP_ADMIN_PASSWORD backend /app/provision_admin
docker compose up -d --wait --wait-timeout 180 backend frontend

./scripts/smoke-test.sh
