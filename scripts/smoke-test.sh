#!/usr/bin/env bash
set -euo pipefail

backend_url="http://127.0.0.1:${BACKEND_PORT:-8000}"
frontend_url="http://127.0.0.1:${FRONTEND_PORT:-3000}"
chroma_url="http://127.0.0.1:${CHROMA_PORT:-8001}"

wait_for_http() {
  local name=$1 url=$2
  local status
  for attempt in $(seq 1 60); do
    status="$(curl --silent --output /dev/null --write-out '%{http_code}' "$url" || true)"
    if [[ "$status" == 200 ]]; then
      printf '%s: HTTP 200\n' "$name"
      return 0
    fi
    sleep 2
  done
  printf '%s did not become ready: %s\n' "$name" "$url" >&2
  return 1
}

wait_for_http 'ChromaDB' "$chroma_url/api/v2/heartbeat"
wait_for_http 'Backend /health' "$backend_url/health"
wait_for_http 'Frontend /' "$frontend_url/"

login_response="$(python3 -c 'import json, os; print(json.dumps({"email": os.environ["FERXARP_ADMIN_EMAIL"], "password": os.environ["FERXARP_ADMIN_PASSWORD"]}))' |
  curl --silent --show-error --header 'Content-Type: application/json' --data-binary @- --write-out '\n%{http_code}' "$backend_url/api/auth/login")"
login_status="${login_response##*$'\n'}"
[[ "$login_status" == 200 ]]
token="$(printf '%s' "${login_response%$'\n'*}" | python3 -c 'import json, sys; print(json.load(sys.stdin)["token"])')"
test -n "$token"
printf 'Admin login: HTTP 200\n'
admin_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' --header "Authorization: Bearer $token" "$backend_url/api/auth/ngos")"
[[ "$admin_status" == 200 ]]
printf 'Protected Admin route: HTTP 200\n'
