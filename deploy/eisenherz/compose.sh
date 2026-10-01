#!/usr/bin/env bash
set -euo pipefail
[[ "$(whoami)" == ferxarp-deploy && "$(hostname)" == eisenherz ]] || { echo 'ABORTADO: IDENTIDAD REMOTA INCORRECTA' >&2; exit 41; }
check_context() {
  local current
  current=$(docker context show) || { echo 'ABORTADO: DOCKER ROOTLESS NO CONFIRMADO' >&2; exit 42; }
  printf '%s\n' "$current" >&2
  [[ "$current" == rootless && -z "${DOCKER_HOST:-}" ]] || { echo 'ABORTADO: DOCKER ROOTLESS NO CONFIRMADO' >&2; exit 42; }
}
check_context
security=$(docker info --format '{{json .SecurityOptions}}') || { echo 'ABORTADO: DOCKER ROOTLESS NO CONFIRMADO' >&2; exit 42; }
[[ "$security" == *rootless* ]] || { echo 'ABORTADO: DOCKER ROOTLESS NO CONFIRMADO' >&2; exit 42; }
check_context
exec docker compose -p ferxarp --env-file /srv/ferxarp/deploy/.env -f /srv/ferxarp/repo/deploy/eisenherz/compose.yml "$@"
