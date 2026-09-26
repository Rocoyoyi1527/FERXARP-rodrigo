#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
phase="${1:-}"
if [[ "$phase" != before && "$phase" != after ]]; then
  echo "Usage: scripts/sonar-scan.sh before|after" >&2
  exit 2
fi
: "${SONAR_TOKEN:?SONAR_TOKEN is required in the environment}"
: "${SONAR_SCANNER_BIN:?Set SONAR_SCANNER_BIN to the official SonarScanner CLI executable}"
export SONAR_HOST_URL="${SONAR_HOST_URL:-http://127.0.0.1:19000}"

for project in frontend backend; do
  if [[ ! -s "$project/coverage/lcov.info" ]]; then
    echo "$project/coverage/lcov.info is required; run coverage first" >&2
    exit 2
  fi
  (cd "$project" && "$SONAR_SCANNER_BIN" -Dsonar.qualitygate.wait=true)
  python3 scripts/sonar-metrics.py "ferxarp-$project" \
    "docs/evidencias/quality/sonar-${phase}-${project}.json"
done
